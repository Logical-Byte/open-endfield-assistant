//! 导航识别和动作共享的模板搜索目标。
//!
//! 模板名称和搜索区域记录游戏 UI 的外部事实，匹配阈值记录当前实现约定。
//! 识别树和导航图共用这些目标，避免同一事实只在其中一处更新。

use crate::{
    automation::TemplateTarget,
    utils::region::{Region2D, ltrb},
};

/// 导航状态识别和动作中所有模板匹配的默认最低得分。
const TEMPLATE_MATCH_THRESHOLD: f32 = 0.75;
/// 档案库主界面、子界面和档案详情页面右上角关闭按钮的模板搜索区域。
const ARCHIVE_CLOSE_ROI: Region2D<u32> = ltrb!(1180, 0, 1280, 100);
/// 档案库子界面左下角分类水印的模板搜索区域。
const ARCHIVE_WATERMARK_ROI: Region2D<u32> = ltrb!(52, 482, 189, 618);

/// 协议终端中档案库入口的模板搜索目标。
pub(super) const TERMINAL_ARCHIVE_ENTRY: TemplateTarget =
    target("档案库.png", ltrb!(971, 108, 1280, 700));
/// 大世界右上角协议终端入口的模板搜索目标。
pub(super) const OVERWORLD_TERMINAL_ENTRY: TemplateTarget =
    target("协议终端.png", ltrb!(1180, 0, 1280, 100));
/// 档案库左上角标题的模板搜索目标。
pub(super) const ARCHIVE_TITLE: TemplateTarget =
    target("情报档案库/情报档案库标题.png", ltrb!(0, 0, 162, 76));
/// 档案详情页面顶部装饰的模板搜索目标。
pub(super) const ARCHIVE_DETAIL_DECORATION: TemplateTarget =
    target("情报档案库/档案详情装饰.png", ltrb!(356, 34, 496, 77));
/// 档案详情页面右上角关闭按钮的模板搜索目标。
pub(super) const ARCHIVE_DETAIL_CLOSE: TemplateTarget =
    target("情报档案库/档案详情关闭.png", ARCHIVE_CLOSE_ROI);
/// 档案库子界面右上角关闭按钮的模板搜索目标。
pub(super) const ARCHIVE_SUBSCENE_CLOSE: TemplateTarget =
    target("情报档案库/档案库子界面关闭.png", ARCHIVE_CLOSE_ROI);
/// 档案库主界面右上角关闭按钮的模板搜索目标。
pub(super) const ARCHIVE_MAIN_CLOSE: TemplateTarget =
    target("情报档案库/档案库主界面关闭.png", ARCHIVE_CLOSE_ROI);
/// 档案库主界面音像存档入口的模板搜索目标。
pub(super) const ARCHIVE_MEDIA_ENTRY: TemplateTarget =
    target("情报档案库/音像存档.png", ltrb!(692, 371, 959, 601));
/// 档案库主界面见闻辑录入口的模板搜索目标。
pub(super) const ARCHIVE_RECORDS_ENTRY: TemplateTarget =
    target("情报档案库/见闻辑录.png", ltrb!(957, 135, 1221, 371));
/// 档案库主界面中枢档案入口的模板搜索目标。
pub(super) const ARCHIVE_CENTRAL_ENTRY: TemplateTarget =
    target("情报档案库/中枢档案.png", ltrb!(958, 369, 1220, 601));
/// 档案库子界面音像存档水印的模板搜索目标。
pub(super) const ARCHIVE_MEDIA_WATERMARK: TemplateTarget =
    target("情报档案库/音像存档水印.png", ARCHIVE_WATERMARK_ROI);
/// 档案库子界面见闻辑录水印的模板搜索目标。
pub(super) const ARCHIVE_RECORDS_WATERMARK: TemplateTarget =
    target("情报档案库/见闻辑录水印.png", ARCHIVE_WATERMARK_ROI);
/// 档案库子界面中枢档案水印的模板搜索目标。
pub(super) const ARCHIVE_CENTRAL_WATERMARK: TemplateTarget =
    target("情报档案库/中枢档案水印.png", ARCHIVE_WATERMARK_ROI);

/// 使用导航的默认阈值构造模板搜索目标。
const fn target(template_name: &'static str, roi: Region2D<u32>) -> TemplateTarget {
    TemplateTarget {
        template_name,
        roi,
        threshold: TEMPLATE_MATCH_THRESHOLD,
    }
}
