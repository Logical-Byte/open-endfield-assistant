export interface MirrorchyanResourcesLatestQueryParams {
  /**
   * 用于指定 CPU 架构类型，若为空则为通用类型。支持以下取值：
   * - 386（x86、x86_32、i386 可用作别名）
   * - amd64（x64、x86_64、intel64 可用作别名）
   * - arm
   * - arm64（aarch64 可用作别名）
   */
  arch?: string;
  /**
   * CDKey
   */
  cdk?: string;
  /**
   * 更新频道，stable | beta | alpha，未填写默认 stable
   */
  channel?: string;
  /**
   * 表示当前的版本名称。
   * 当提供此参数时，服务器会检查该版本是否在镜像库中存在。如果存在，则返回增量更新；如果不存在，则返回全量更新。
   * 如果未提供此参数，则默认返回全量更新。
   */
  current_version?: string;
  /**
   * 用于指定操作系统类型，若为空则为通用类型。支持以下取值：
   * - windows（win、win32可用作别名）
   * - linux
   * - darwin（macos、mac、osx可用作别名）
   * - android
   */
  os?: string;
  /**
   * 客户端标识，可用于营收统计来源
   */
  user_agent?: string;
}

export interface MirrorchyanApiResponse<T> {
  /**
   * 响应代码，https://github.com/MirrorChyan/docs/blob/main/ErrorCode.md
   */
  code: number;
  /**
   * 响应数据
   */
  data?: T;
  /**
   * 响应信息
   */
  msg: string;
}

/**
 * 响应数据
 */
export interface MirrorchyanResourcesLatestResponseData {
  /**
   * 更新包架构
   */
  arch: string;
  /**
   * CDK过期时间戳
   */
  cdk_expired_time?: number;
  /**
   * 更新频道，stable | beta | alpha
   */
  channel: string;
  /**
   * 自定义数据
   */
  custom_data?: string;
  /**
   * 文件大小
   */
  filesize?: number;
  /**
   * 更新包系统
   */
  os: string;
  /**
   * 发版日志
   */
  release_note: string;
  /**
   * sha256
   */
  sha256?: string;
  /**
   * 更新包类型，incremental | full
   */
  update_type?: string;
  /**
   * 下载地址
   */
  url?: string;
  /**
   * 资源版本名称
   */
  version_name: string;
  /**
   * 资源版本号仅内部使用
   */
  version_number: number;
}

export type MirrorchyanResourcesLatestResponse =
  MirrorchyanApiResponse<MirrorchyanResourcesLatestResponseData>;
