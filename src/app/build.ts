import { AppVariant } from '@/shared/types/app';

export { AppVariant };

/** 前端构建时独立选择 OEA dev 或 OEA release，不通过 IPC 读取后端身份。 */
export const appVariant: AppVariant = __OEA_APP_VARIANT__;

/** 当前应用版本号，由构建配置注入。 */
export const appVersion: string = __OEA_VERSION__;
