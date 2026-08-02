//! 共享应用状态（模块级单例 ref，多个组件共享）。
import { ref } from 'vue'
import { getStatus, onAppStatus } from './tauri'

/** 主任务是否正在运行 */
export const running = ref(false)

let initialized = false

/** 初始化：拉取一次状态并订阅状态变更事件（幂等，全局只需调用一次）。 */
export function initAppState(): void {
  if (initialized) return
  initialized = true
  getStatus().then((s) => {
    running.value = s.running
  })
  onAppStatus((s) => {
    running.value = s.running
  })
}
