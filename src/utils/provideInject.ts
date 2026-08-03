import type { PreviewTarget } from '@/composables/image-preview/useImagePreview';
import type { InjectionKey } from 'vue';

export const openImagePreviewKey = Symbol() as InjectionKey<(target: PreviewTarget) => void>;
