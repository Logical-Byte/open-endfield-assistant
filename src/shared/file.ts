/**
 * 文件下载工具函数
 * @param href 文件链接或数据链接 (Blob, DataURL, URL)
 * @param filename 下载的文件名
 */
function triggerDownload(href: string, filename: string) {
  const a = document.createElement('a');
  a.href = href;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
}

/**
 * 触发浏览器下载文件
 * @param urlOrBlob 文件链接、数据链接或 Blob 对象
 * @param filename 下载的文件名
 */
export async function downloadFile(urlOrBlob: string | Blob, filename: string): Promise<void> {
  // 如果是 Blob，创建对象 URL 后下载
  if (urlOrBlob instanceof Blob) {
    const objectUrl = URL.createObjectURL(urlOrBlob);
    triggerDownload(objectUrl, filename);
    setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
    return;
  }

  const url = urlOrBlob;

  // blob: / data: URL 可直接触发下载，无需重新 fetch
  if (url.startsWith('blob:') || url.startsWith('data:')) {
    return triggerDownload(url, filename);
  }

  // 判断是否同源
  const isSameOrigin = (() => {
    try {
      return new URL(url, location.href).origin === location.origin;
    } catch {
      return true;
    }
  })();

  // 同源资源：直接触发下载
  if (isSameOrigin) {
    return triggerDownload(url, filename);
  }

  // 跨域资源：download 属性会被忽略，尝试转换为 blob 下载
  try {
    const res = await fetch(url, { referrerPolicy: 'no-referrer' });
    if (!res.ok) throw new Error();
    const blob = await res.blob();
    return downloadFile(blob, filename);
  } catch {
    // 最终回退：新标签页打开
    window.open(url, '_blank', 'noopener,noreferrer');
  }
}
