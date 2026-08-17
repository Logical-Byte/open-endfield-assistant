<script setup lang="ts">
import { oeaConfig } from '@/utils/app/config';
import { UPDATE_PROXY_MODE_ITEMS, UPDATE_SOURCE_ITEMS } from '@/utils/update-options';
</script>

<template>
  <div class="grid grid-cols-[auto_1fr] items-center gap-2 border-t border-default pt-3">
    <span class="text-xs text-toned">下载源</span>
    <USelect v-model="oeaConfig.updateSource" :items="UPDATE_SOURCE_ITEMS" size="sm" />
    <template v-if="oeaConfig.updateSource === 'mirrorchyan'">
      <span class="text-xs text-toned">CDK</span>
      <UInput v-model="oeaConfig.mirrorchyanCdk" size="sm" type="password" />
    </template>
    <span class="text-xs text-toned">代理</span>
    <USelect v-model="oeaConfig.updateProxyMode" :items="UPDATE_PROXY_MODE_ITEMS" size="sm" />
    <template v-if="oeaConfig.updateProxyMode === 'custom'">
      <span class="text-xs text-toned">地址</span>
      <UInput v-model="oeaConfig.updateProxyUrl" placeholder="http://127.0.0.1:7890" size="sm" />
    </template>
  </div>
</template>
