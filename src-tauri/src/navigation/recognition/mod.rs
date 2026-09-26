//! 从一张不可变截图逐层判定具体 UI 状态。
//!
//! [`UiStateGroup`] 的静态实现组成一棵人工设计的判定树。每个节点只负责当前层的
//! 探测和分支；状态组本身不会进入导航图。整次判定共享同一个
//! [`RecognitionContext`]，探测失败产生 [`Recognition::Unrecognized`]，模板加载或
//! 图像处理错误则通过 `Err` 传播。
//!
//! 更一般地，每个具体 UI 状态都可以看成同一张截图上一组确定性 `probe` 结果的谓词，
//! 而状态组树只是其中一份人工编排的静态执行计划。查询约束未来可以允许提前停止，
//! 概率提示也只能调整 `probe` 顺序，不能排除状态或改变最终答案。本实现暂不建立通用
//! `probe` 缓存、代价模型或动态计划生成器。

mod archive;

use anyhow::{Context, Result};
use image::RgbaImage;

use crate::automation::{ScreenCapture, TemplateMatching, TemplateTarget};

use self::archive::{ARCHIVE_PAGES, recognize_archive_detail, recognizes_archive_title};
use super::{
    state::UiState,
    targets::{OVERWORLD_TERMINAL_ENTRY, TERMINAL_ARCHIVE_ENTRY},
};

/// 全部已知 UI 状态的根状态组。
///
/// 该组包含大世界、协议终端、档案库主界面、六个档案库子界面和
/// 档案详情页面。一次细化可以产出具体状态“档案详情页面”、“协议终端”或
/// “大世界”，或继续保留状态组“档案库导航页”。所有顶层判定条件均不满足时
/// 产出未识别结果。
///
/// 判定顺序是语义的一部分：档案详情页面优先于档案库导航页，随后依次判断协议终端和
/// 大世界。若意外有多个顶层条件同时成立，返回排在最前面的具体 UI 状态。
static ANY_UI: AnyUi = AnyUi;

/// 一次 UI 状态识别的最终结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Recognition {
    /// 已经判定出一个具体 UI 状态。
    Determined(UiState),
    /// 当前截图不满足任何已知具体 UI 状态的判定条件。
    Unrecognized,
}

/// 一次状态组细化的结果。
enum Refinement {
    /// 已经判定出一个具体 UI 状态。
    Determined(UiState),
    /// 当前证据只能缩小候选集，继续细化指定的下一层状态组。
    StillVague(&'static dyn UiStateGroup),
    /// 当前截图不满足该状态组中任何已知具体 UI 状态的判定条件。
    Unrecognized,
}

/// 固定判定树中的一个状态组节点。
trait UiStateGroup: Sync {
    /// 用于错误上下文和调试日志的状态组名称。
    fn name(&self) -> &'static str;

    /// 使用同一张截图把当前状态集合细化一层。
    fn refine(&'static self, cx: &mut RecognitionContext<'_>) -> Result<Refinement>;
}

/// 一次识别共享的截图和探测能力。
struct RecognitionContext<'a> {
    screenshot: &'a RgbaImage,
    templates: &'a mut dyn TemplateMatching,
}

impl RecognitionContext<'_> {
    fn matches(&mut self, target: &TemplateTarget) -> Result<bool> {
        Ok(self
            .templates
            .find_template(self.screenshot, target)?
            .is_some())
    }
}

/// 获取一张截图并运行完整的固定判定树。
pub(super) fn recognize<C>(cx: &mut C) -> Result<Recognition>
where
    C: ScreenCapture + TemplateMatching,
{
    let screenshot = cx.screenshot()?;
    let mut recognition = RecognitionContext {
        screenshot: &screenshot,
        templates: cx,
    };
    let mut group: &'static dyn UiStateGroup = &ANY_UI;

    loop {
        match group
            .refine(&mut recognition)
            .with_context(|| format!("UI 状态组 {} 判定失败", group.name()))?
        {
            Refinement::Determined(state) => return Ok(Recognition::Determined(state)),
            Refinement::StillVague(next) => group = next,
            Refinement::Unrecognized => return Ok(Recognition::Unrecognized),
        }
    }
}

/// 全部已知 UI 状态的根状态组。
///
/// 该组包含大世界、协议终端、档案库主界面、六个档案库子界面和
/// 档案详情页面。一次细化可以产出具体状态“档案详情页面”、“协议终端”或
/// “大世界”，或继续保留状态组“档案库导航页”。所有顶层判定条件均不满足时
/// 产出未识别结果。
///
/// 判定顺序是语义的一部分：档案详情页面优先于档案库导航页，随后依次判断协议终端和
/// 大世界。若意外有多个顶层条件同时成立，返回排在最前面的具体 UI 状态。
struct AnyUi;

impl UiStateGroup for AnyUi {
    fn name(&self) -> &'static str {
        "全部 UI"
    }

    fn refine(&'static self, cx: &mut RecognitionContext<'_>) -> Result<Refinement> {
        if recognize_archive_detail(cx)? {
            return Ok(Refinement::Determined(UiState::archive_detail()));
        }
        if recognizes_archive_title(cx)? {
            return Ok(Refinement::StillVague(&ARCHIVE_PAGES));
        }

        if cx.matches(&TERMINAL_ARCHIVE_ENTRY)? {
            return Ok(Refinement::Determined(UiState::Terminal));
        }

        if cx.matches(&OVERWORLD_TERMINAL_ENTRY)? {
            return Ok(Refinement::Determined(UiState::Overworld));
        }

        Ok(Refinement::Unrecognized)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use image::{Rgba, RgbaImage};

    use crate::{
        automation::{ScreenCapture, TemplateMatch, TemplateMatching, TemplateTarget},
        navigation::state::{ArchiveSubscene, RecordsPage, UiState},
        utils::region::Region2D,
    };

    use super::{Recognition, recognize};

    struct TestAutomation {
        screenshot: RgbaImage,
        matching_templates: Vec<&'static str>,
        fail_template: Option<&'static str>,
    }

    impl TestAutomation {
        fn matching(templates: &[&'static str]) -> Self {
            Self {
                screenshot: RgbaImage::from_pixel(1280, 720, Rgba([255, 255, 255, 255])),
                matching_templates: templates.to_vec(),
                fail_template: None,
            }
        }

        fn darken(&mut self, region: Region2D<u32>) {
            for y in region.y0()..region.y1() {
                for x in region.x0()..region.x1() {
                    self.screenshot.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                }
            }
        }
    }

    impl ScreenCapture for TestAutomation {
        fn screenshot(&mut self) -> Result<RgbaImage> {
            Ok(self.screenshot.clone())
        }
    }

    impl TemplateMatching for TestAutomation {
        fn find_template(
            &mut self,
            _screenshot: &RgbaImage,
            target: &TemplateTarget,
        ) -> Result<Option<TemplateMatch>> {
            if self.fail_template == Some(target.template_name) {
                anyhow::bail!("模板损坏");
            }
            Ok(self
                .matching_templates
                .contains(&target.template_name)
                .then_some(TemplateMatch {
                    region: target.roi,
                    score: 1.0,
                }))
        }
    }

    #[test]
    fn recognizes_overworld_through_the_root_group() {
        let mut automation = TestAutomation::matching(&["协议终端.png"]);

        let result = recognize(&mut automation).unwrap();

        assert_eq!(result, Recognition::Determined(UiState::Overworld));
    }

    #[test]
    fn returns_unrecognized_when_no_known_state_matches() {
        let mut automation = TestAutomation::matching(&[]);

        let result = recognize(&mut automation).unwrap();

        assert_eq!(result, Recognition::Unrecognized);
    }

    #[test]
    fn records_all_light_falls_back_to_paper() {
        let mut automation = TestAutomation::matching(&[
            "情报档案库/情报档案库标题.png",
            "情报档案库/档案库子界面关闭.png",
            "情报档案库/见闻辑录水印.png",
        ]);

        let result = recognize(&mut automation).unwrap();

        assert_eq!(
            result,
            Recognition::Determined(UiState::archive_subscene(ArchiveSubscene::Records(
                RecordsPage::Paper,
            )))
        );
    }

    #[test]
    fn archive_detail_precedes_archive_navigation_pages() {
        let mut automation = TestAutomation::matching(&[
            "情报档案库/档案详情装饰.png",
            "情报档案库/档案详情关闭.png",
            "情报档案库/情报档案库标题.png",
            "情报档案库/档案库子界面关闭.png",
            "情报档案库/见闻辑录水印.png",
        ]);

        let result = recognize(&mut automation).unwrap();

        assert_eq!(result, Recognition::Determined(UiState::archive_detail()));
    }

    #[test]
    fn records_multiple_dark_tabs_choose_the_topmost() {
        let mut automation = TestAutomation::matching(&[
            "情报档案库/情报档案库标题.png",
            "情报档案库/档案库子界面关闭.png",
            "情报档案库/见闻辑录水印.png",
        ]);
        automation.darken(Region2D::from_ltwh(180, 120, 60, 36));
        automation.darken(Region2D::from_ltwh(180, 184, 60, 36));

        let result = recognize(&mut automation).unwrap();

        assert_eq!(
            result,
            Recognition::Determined(UiState::archive_subscene(ArchiveSubscene::Records(
                RecordsPage::Paper,
            )))
        );
    }

    #[test]
    fn propagates_probe_errors_instead_of_returning_unrecognized() {
        let mut automation = TestAutomation::matching(&[]);
        automation.fail_template = Some("情报档案库/档案详情装饰.png");

        let error = recognize(&mut automation).unwrap_err();

        assert!(format!("{error:#}").contains("模板损坏"));
    }
}
