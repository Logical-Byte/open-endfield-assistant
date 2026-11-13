//! 检查、下载及安装的高层编排。
use crate::{
    CheckOptions, Error, Reason, check, download, http,
    install::{self, HelperLaunch, HelperStarted, InstallStage, InstallTarget},
    manager::{UpdateInfo, UpdateManager, UpdateStatus},
    source,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{debug, error, info, warn};

/// 持有更新的可用版本、下载会话与待安装状态；操作租约不暴露给宿主。
#[derive(Default)]
pub struct Updater {
    manager: UpdateManager,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateAvailability {
    UpToDate,
    Available { update: UpdateInfo },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadOutcome {
    Completed { update: UpdateInfo },
    Cancelled,
}
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded_size: u64,
    pub total_size: u64,
    pub speed: u64,
    pub progress: f64,
}
/// 本次下载独享的进度接收器。投递失败只记录一次，不中止下载。
pub type ProgressSink = Arc<dyn Fn(DownloadProgress) -> Result<(), String> + Send + Sync>;

impl Updater {
    pub fn status(&self) -> UpdateStatus {
        self.manager.status()
    }
    pub fn is_installing(&self) -> bool {
        self.manager.is_installing()
    }
    pub fn cancel_download(&self) -> Result<(), Error> {
        self.manager.cancel_download()
    }

    /// 选择并暂存本地包，返回后释放操作租约，由宿主等待用户确认。
    pub fn select_local_package(
        &self,
        choose_package: impl FnOnce() -> Result<Option<PathBuf>, Error>,
    ) -> Result<Option<PathBuf>, Error> {
        let _lease = self.manager.start_developer_install()?;
        choose_package()
    }
    /// 检查更新并缓存规范化后的可用更新。
    /// 取得操作租约后才调用 `options`，由宿主读取本次生效设置快照。
    pub async fn check(
        &self,
        current_version: &str,
        options: impl FnOnce() -> Result<CheckOptions, Error>,
    ) -> Result<UpdateAvailability, Error> {
        let check_lease = self.manager.start_check().inspect_err(|check_error| {
            warn!(
                operation = "check",
                error = %check_error,
                "更新检查请求被状态机拒绝"
            );
        })?;
        let settings = options()?;
        let user_agent = http::update_user_agent(current_version);
        debug!(
            operation = "check",
            current_version = %current_version,
            proxy_configured = settings.proxy_url.is_some(),
            "更新检查开始"
        );
        info!(
            "开始检查更新：当前版本 {}，{}",
            current_version,
            if settings.proxy_url.is_some() {
                "使用代理"
            } else {
                "不使用代理"
            }
        );
        let available = match check::check_for_update(&settings, current_version, &user_agent).await
        {
            Ok(available) => available,
            Err(check_error) => {
                error!(
                    operation = "check",
                    current_version,
                    error = %check_error,
                    "更新检查失败"
                );
                return Err(check_error);
            }
        };

        match available {
            Some(metadata) => {
                let result = UpdateAvailability::Available {
                    update: UpdateInfo {
                        version_name: metadata.version_name.clone(),
                        release_note: metadata.release_note.clone(),
                    },
                };
                debug!(
                    operation = "check",
                    result = "available",
                    current_version = %current_version,
                    latest_version = %metadata.version_name,
                    "更新检查完成"
                );
                info!(
                    "发现新版本 {}（当前版本 {}）",
                    metadata.version_name, current_version
                );
                check_lease.complete(Some(metadata));
                Ok(result)
            }
            None => {
                debug!(
                    operation = "check",
                    result = "up_to_date",
                    current_version = %current_version,
                    "更新检查完成"
                );
                info!("当前已是最新版本 {current_version}");
                check_lease.complete(None);
                Ok(UpdateAvailability::UpToDate)
            }
        }
    }

    /// 使用缓存的可用更新和当前设置完成更新包下载。
    /// 接受下载操作后由宿主根据缓存元数据和当前设置选择下载源，内核不执行源回退。
    ///
    /// 若下载结果将交给 [`Self::install`]，其规范化路径必须位于安装目标根目录的
    /// `cache/downloads` 内。建议将 `download_dir` 设为该目录，避免下载完成后因
    /// 安装路径校验失败而无法安装。
    pub async fn download(
        &self,
        current_version: &str,
        download_dir: &Path,
        select_source: impl FnOnce(&source::AvailableUpdateMetadata) -> Result<source::Source, Error>,
        on_progress: ProgressSink,
    ) -> Result<DownloadOutcome, Error> {
        let download_lease =
            self.manager
                .start_update_download()
                .inspect_err(|download_error| {
                    warn!(
                        operation = "download",
                        error = %download_error,
                        "更新下载请求被状态机拒绝"
                    );
                })?;
        let metadata = download_lease.available_update().clone();
        let selected = select_source(&metadata)?;
        let session = download_lease.session();
        let session_id = download_lease.id();
        let cancellation = session.cancellation();
        let user_agent = http::update_user_agent(current_version);
        debug!(
            operation = "download",
            session_id,
            version = %metadata.version_name,
            source = selected.label(),
            "更新下载请求已接受"
        );
        info!(
            "开始下载更新 {}：下载源为 {}",
            metadata.version_name,
            selected.label(),
        );
        let plan =
            match source::resolve_download_plan(&metadata, selected, &user_agent, &cancellation)
                .await
            {
                Ok(plan) => plan,
                Err(error) if error.reason == Reason::Cancelled => {
                    return Ok(DownloadOutcome::Cancelled);
                }
                Err(download_error) => {
                    error!(
                        operation = "download",
                        phase = "resolve_plan",
                        session_id,
                        version = %metadata.version_name,
                        error = %download_error,
                        "更新下载失败"
                    );
                    return Err(download_error);
                }
            };
        let package_path = match download::download_update_plan(
            plan,
            session_id,
            session,
            &user_agent,
            download_dir,
            on_progress,
        )
        .await
        {
            Ok(package_path) => package_path,
            Err(error) if error.reason == Reason::Cancelled => {
                debug!(
                    operation = "download",
                    result = "cancelled",
                    session_id,
                    version = %metadata.version_name,
                    "更新下载已取消"
                );
                info!("更新 {} 的下载已取消", metadata.version_name);
                return Ok(DownloadOutcome::Cancelled);
            }
            Err(download_error) => {
                error!(operation = "download", phase = "transfer", session_id,
                version = %metadata.version_name, error = %download_error, "更新下载失败");
                return Err(download_error);
            }
        };

        let update = UpdateInfo::from(&metadata);
        debug!(
            operation = "download",
            result = "pending_install",
            session_id,
            version = %metadata.version_name,
            package_path = %package_path.display(),
            "更新包已登记为待安装更新"
        );
        info!("更新 {} 下载完成，等待安装", metadata.version_name);
        download_lease.complete(package_path);
        Ok(DownloadOutcome::Completed { update })
    }

    /// helper 启动成功后保持安装状态，宿主应立即请求退出。
    /// `helper` 在事务准备完成后调用，宿主在其中提供源路径和原生参数快照。
    pub fn install(
        &self,
        target: &InstallTarget,
        helper: impl FnOnce() -> Result<HelperLaunch, String>,
        on_stage: impl Fn(InstallStage),
    ) -> Result<HelperStarted, Error> {
        let lease = self.manager.start_install().inspect_err(|error| {
            warn!(operation = "install", error = %error, "更新安装请求被状态机拒绝");
        })?;
        debug!(operation = "install", package = %lease.package_path().display(), "更新安装开始");
        info!("开始安装更新");
        let started =
            install::install_update_inner(target, helper, lease.package_path(), &on_stage)
                .inspect_err(
                    |error| error!(operation = "install", error = %error, "更新安装失败"),
                )?;
        lease.complete();
        Ok(started)
    }

    /// 在互斥安装状态中让宿主选择并暂存本地更新包。
    ///
    /// `choose_package` 返回 `Ok(None)` 表示取消选择，会释放安装状态。
    /// 返回 `Ok(Some(path))` 时，文件的规范化路径必须位于 `target` 根目录的
    /// `cache/downloads` 内，且须是宿主允许内核删除的暂存 ZIP。
    /// 内核会在准备完成后删除该文件，也会在部分准备失败路径尝试删除它。
    /// 需要保留原始 ZIP 时，宿主应先复制到暂存目录，不能依赖安装失败后文件仍存在。
    ///
    /// `helper` 提供的启动配置须满足 [`HelperLaunch`] 的执行协议。
    /// 成功返回 `Some(HelperStarted)` 后保持安装状态，宿主须立即请求退出，
    /// 释放目标 executable 的占用。
    pub fn install_local(
        &self,
        choose_package: impl FnOnce() -> Result<Option<PathBuf>, Error>,
        target: &InstallTarget,
        helper: impl FnOnce() -> Result<HelperLaunch, String>,
        on_stage: impl Fn(InstallStage),
    ) -> Result<Option<HelperStarted>, Error> {
        let lease = self.manager.start_developer_install().inspect_err(|error| {
            warn!(operation = "install", install_kind = "developer_package", error = %error, "开发者更新安装请求被状态机拒绝");
        })?;
        debug!(
            operation = "install",
            install_kind = "developer_package",
            "开发者更新包选择流程开始"
        );
        info!("请选择用于安装的开发者更新包");
        let package = match choose_package() {
            Ok(Some(package)) => package,
            Ok(None) => return Ok(None),
            Err(install_error) => {
                error!(operation = "install", install_kind = "developer_package", error = %install_error, "开发者更新包选择或暂存失败");
                return Err(install_error);
            }
        };
        let started = install::install_update_inner(target, helper, &package, &on_stage)
            .inspect_err(
                |error| error!(operation = "install", install_kind = "developer_package", error = %error, "开发者更新安装失败"),
            )?;
        lease.complete();
        Ok(Some(started))
    }
}
