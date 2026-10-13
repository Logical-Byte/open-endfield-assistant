/// <reference types="vite/client" />

/** 构建期注入的应用版本号（来自 `src-tauri/tauri.conf.json`）。 */
declare const __OEA_VERSION__: string;

/** 构建时注入 OEA dev 或 OEA release 对应的枚举值。 */
declare const __OEA_APP_VARIANT__: import('./shared/types/app').AppVariant;
