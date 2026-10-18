<template>
  <UCard
    class="rounded-none border-b border-default bg-transparent ring-0"
    :ui="{
      header: 'flex min-h-14 items-center p-3 sm:px-3 [&>*]:w-full',
      body: 'p-3 sm:p-3',
      footer: 'p-3 sm:px-3',
    }"
  >
    <template v-if="$slots.header" #header><slot name="header" /></template>
    <slot />
  </UCard>
</template>
