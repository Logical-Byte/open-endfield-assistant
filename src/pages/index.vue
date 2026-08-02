<script setup lang="ts">
import { initAppState } from '../lib/appState'

initAppState()
</script>

<template>
  <div class="flex flex-col gap-6">
    <div>
      <h1 class="text-2xl font-semibold">明日方舟：终末地 档案库扫描助手</h1>
      <p class="mt-1 text-muted">
        通过快捷键或下方按钮控制档案库扫描；启动后自动导航并扫描全部 6 个子分类。
      </p>
    </div>

    <StatusCard />
    <ControlPanel />
    <LogViewer />
  </div>
</template>
