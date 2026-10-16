import type { MaybeRefOrGetter } from 'vue';
import { shallowReadonly, shallowRef } from 'vue';

/** 业务调用方提交给应用级图片预览器的目标。 */
export type ImagePreviewTarget = {
  /** 要显示的图片地址。 */
  readonly url: string;
  /** 图片标题和替代文本。 */
  readonly name: string;
  /** 下载时求值的文件名，允许随业务数据变化。 */
  readonly downloadName: MaybeRefOrGetter<string>;
};

/** 当前应用唯一的图片预览目标，由本模块集中修改。 */
const previewTarget = shallowRef<ImagePreviewTarget | null>(null);

/** 供唯一图片预览宿主消费的只读目标。 */
export const currentImagePreviewTarget = shallowReadonly(previewTarget);

/** 业务调用方用此入口请求应用级图片预览器显示目标。 */
export function openImagePreview(target: ImagePreviewTarget): void {
  // 总是创建新对象，重复打开同一个目标时也会触发宿主重置视图。
  previewTarget.value = { ...target };
}

/** 供图片预览宿主响应关闭交互，不作为业务调用方的公共入口。 */
export function closeImagePreview(): void {
  previewTarget.value = null;
}
