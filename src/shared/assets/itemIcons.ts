import type { IconId } from '@/shared/types/essence';

/** Vite 从 resources/assets 提供图片，开发和打包共用同一条 URL。 */
export function itemIconUrl(id: IconId): string {
  return `${import.meta.env.BASE_URL}itemicon/${encodeURIComponent(id)}.png`;
}
