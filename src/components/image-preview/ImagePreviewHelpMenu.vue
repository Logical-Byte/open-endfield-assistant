<template>
  <div class="space-y-4">
    <div class="flex items-center gap-2 font-bold text-highlighted">
      <UIcon class="size-5 text-primary" name="i-lucide-keyboard" />
      快捷键与操作指南
    </div>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">视图控制</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">平移视图</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="W" />
          <UKbd value="A" />
          <UKbd value="S" />
          <UKbd value="D" />
          <UKbd value="arrowup" />
          <UKbd value="arrowleft" />
          <UKbd value="arrowdown" />
          <UKbd value="arrowright" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">平移图像</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd>
            <UIcon class="me-0.5" name="i-lucide-mouse" />
            拖动
          </UKbd>
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">旋转控制</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">顺时针旋转 90°</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="R" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">逆时针旋转 90°</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="shift" /> + <UKbd value="R" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">平滑旋转</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="Q" />
          <UKbd value="E" />
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">缩放控制</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">缩放图像</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="-" />
          <UKbd value="=" />
          <UKbd>
            <UIcon class="me-0.5" name="i-lucide-mouse" />
            滚轮
          </UKbd>
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">平滑缩放</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="[" />
          <UKbd value="]" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">缩放至预设大小</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd v-for="x in 9" :key="x" :value="x.toString()" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">重置视图</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="0" />
        </div>
      </div>
    </section>

    <USeparator />

    <section class="space-y-1.5">
      <p class="text-xs font-medium tracking-wider text-muted">高级操作</p>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">加速平移 / 旋转 / 缩放</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="shift" /> + <UKbd value="操作键" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">下载图像</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="meta" /> + <UKbd value="S" />
        </div>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-sm text-toned">新标签页打开</span>
        <div class="flex items-center gap-0.5 text-xs text-toned">
          <UKbd value="O" />
        </div>
      </div>
    </section>

    <USeparator />

    <p class="text-center text-xs text-warning">快捷键请在英文输入法下使用</p>
  </div>
</template>
