<script setup lang="ts">
import { computed } from 'vue'
import { useHead } from '@unhead/vue'
import { useColorMode } from '@vueuse/core'

const colorMode = useColorMode()
const themeColor = computed(() => colorMode.value === 'dark' ? '#18181b' : '#ffffff')

useHead({
  meta: [
    { name: 'theme-color', content: themeColor }
  ]
})
</script>

<template>
  <Suspense>
    <UApp>
      <UHeader>
        <template #left>
          <p class="text-base font-semibold">OEA</p>
        </template>

        <template #right>
          <UColorModeButton />

          <UButton
            to="https://github.com/Logical-Byte/open-endfield-assistant"
            target="_blank"
            icon="simple-icons:github"
            aria-label="GitHub"
            color="neutral"
            variant="ghost"
          />
        </template>
      </UHeader>

      <UMain>
        <RouterView />
      </UMain>

      <USeparator icon="simple-icons:vuedotjs" />

      <UFooter>
        <template #left>
          <p class="text-sm text-muted">
            Built with Nuxt UI • © {{ new Date().getFullYear() }}
          </p>
        </template>

        <template #right>
          <UButton
            to="https://github.com/nuxt-ui-templates/starter-vue"
            target="_blank"
            icon="simple-icons:github"
            aria-label="GitHub"
            color="neutral"
            variant="ghost"
          />
        </template>
      </UFooter>
    </UApp>
  </Suspense>
</template>
