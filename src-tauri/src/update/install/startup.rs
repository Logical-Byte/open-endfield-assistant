//! v2 启动阶段提交 candidate/resources。
//!
//! helper 只移动 exe。新 exe 启动后在 Tauri、资源消费者和正常 GUI 初始化之前调用
//! [`complete_startup_transaction`]。资源提交是两个目录 rename：先让旧目录进入
//! `discard/resources`，再把 `candidate/resources` 放到根目录。每个 rename 都是
//! 原子的；第二步失败时 transaction 和 candidate 仍在，下一次启动可以根据目录状态
//! 继续完成，不需要旧版本回滚。

use std::{fs, io::ErrorKind};

use serde::Serialize;
use tracing::{info, warn};

use crate::platform::update::{UpdatePrompt, UpdatePromptMode};

use super::workspace::UpdateWorkspace;

/// 启动阶段对更新事务的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupUpdateResult {
    /// 没有 transaction，正常启动。
    NoTransaction,
    /// exe 仍在 candidate，说明 helper 尚未提交，当前旧 exe 忽略事务并正常启动。
    WaitingForHelper,
    /// v2 已经把 resources 提交到根目录并删除 transaction。
    Completed,
}

/// 尝试在应用初始化前完成资源事务。
///
/// 若 candidate exe 仍存在，返回 `WaitingForHelper`，不阻塞旧版启动。若 exe 已被
/// helper 移走，则按照 candidate 的状态继续 resources 提交。锁竞争返回错误，调用
/// 方应提示用户由另一个进程正在处理事务并退出本次启动。
pub fn complete_startup_transaction(
    workspace: &UpdateWorkspace,
) -> Result<StartupUpdateResult, String> {
    if workspace.read_transaction()?.is_none() {
        // helper 失败会删除 transaction/candidate，但自身副本不能在运行中删除；由后续
        // 启动在没有事务时尽力清理这个副本。
        workspace.best_effort_cleanup();
        return Ok(StartupUpdateResult::NoTransaction);
    }

    let Some(_lock) = workspace
        .try_lock()
        .map_err(|error| format!("打开 transaction.lock 失败: {error}"))?
    else {
        return Err("更新事务正在由另一个进程处理".to_string());
    };

    // helper 可能在我们第一次观察 transaction 后完成失败清理；拿到锁后重新读取，
    // 避免把已经取消的事务误判成“exe 已提交但 resources 缺失”。
    if workspace.read_transaction()?.is_none() {
        workspace.best_effort_cleanup();
        return Ok(StartupUpdateResult::NoTransaction);
    }

    // helper 还没有提交 exe 时，旧版 OEA 忽略 transaction，允许用户正常启动。
    if workspace
        .candidate_path()
        .join(workspace.executable_name())
        .exists()
    {
        return Ok(StartupUpdateResult::WaitingForHelper);
    }

    let mut prompt = UpdatePrompt::new(
        "OEA 更新",
        "正在完成资源更新，请稍候…",
        UpdatePromptMode::from_environment(),
    );
    if let Err(error) = commit_candidate_resources(workspace) {
        prompt.show_error("OEA 更新失败", &error);
        return Err(error);
    }
    workspace
        .remove_transaction()
        .map_err(|error| format!("资源已提交，但删除 transaction 失败: {error}"))?;

    // transaction 删除是提交点；旧目录的递归删除不再影响新版本可启动性。
    workspace.best_effort_cleanup();
    prompt.finish();
    info!("更新事务已完成: resources 已提交");
    Ok(StartupUpdateResult::Completed)
}

fn commit_candidate_resources(workspace: &UpdateWorkspace) -> Result<(), String> {
    let root_resources = workspace.resources_path();
    let candidate_resources = workspace.candidate_path().join("resources");
    let discard_resources = workspace.discard_path().join("resources");

    if candidate_resources.exists() {
        if !candidate_resources.is_dir() {
            return Err(format!(
                "candidate/resources 不是目录: {}",
                candidate_resources.display()
            ));
        }

        // 上一次启动可能已经成功让旧目录位于 discard；此时根目录应为空，直接重试第二步。
        if root_resources.exists() {
            if discard_resources.exists() {
                return Err(format!(
                    "resources 与 discard/resources 同时存在，事务状态不一致: {}",
                    root_resources.display()
                ));
            }
            fs::create_dir_all(workspace.discard_path())
                .map_err(|error| format!("创建 discard 目录失败: {error}"))?;
            fs::rename(&root_resources, &discard_resources).map_err(|error| {
                format!(
                    "原子移动旧 resources 失败 [{}] -> [{}]: {error}",
                    root_resources.display(),
                    discard_resources.display()
                )
            })?;
        } else if !discard_resources.exists() {
            return Err("根目录 resources 与 discard/resources 都不存在，无法继续事务".to_string());
        }

        fs::rename(&candidate_resources, &root_resources).map_err(|error| {
            format!(
                "原子移动 candidate resources 失败 [{}] -> [{}]: {error}",
                candidate_resources.display(),
                root_resources.display()
            )
        })?;
    } else if !root_resources.is_dir() {
        return Err("candidate/resources 已不存在且根目录 resources 也不存在".to_string());
    }

    if workspace.candidate_path().exists() {
        let mut entries = fs::read_dir(workspace.candidate_path())
            .map_err(|error| format!("读取 candidate 状态失败: {error}"))?;
        if entries.next().is_some() {
            return Err("resources 已提交但 candidate 仍有未预期的条目".to_string());
        }
        if let Err(error) = fs::remove_dir(workspace.candidate_path()) {
            if error.kind() != ErrorKind::NotFound {
                warn!("清理空 candidate 目录失败: {error}");
            }
        }
    }
    Ok(())
}
