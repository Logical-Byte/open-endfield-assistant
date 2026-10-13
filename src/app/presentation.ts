import { appVariant, AppVariant, appVersion } from '@/app/build';
import { t } from '@/shared/i18n';

/** 各应用形态共用的产品展示规则。 */
interface BaseAppPresentation {
  /** 产品名称，用于主页左上角和标题栏。 */
  readonly name: string;
  /** 获取页面标题。调用时翻译，以响应界面语言切换。 */
  readonly title: () => string;
}

/** OEA dev 的展示规则，不提供发行版本号。 */
export interface DevAppPresentation extends BaseAppPresentation {
  /** 判别字段，用于收窄到 OEA dev 的展示类型。 */
  readonly variant: AppVariant.Dev;
}

/** OEA release 的展示规则，包含发行版本号。 */
export interface ReleaseAppPresentation extends BaseAppPresentation {
  /** 判别字段，用于收窄到 OEA release 的展示类型。 */
  readonly variant: AppVariant.Release;
  /** 展示版本号，不包含 v 前缀。 */
  readonly version: string;
}

/** 当前应用形态的产品展示规则，通过 variant 区分可用字段。 */
export type AppPresentation = DevAppPresentation | ReleaseAppPresentation;

const presentations: Record<AppVariant, AppPresentation> = {
  [AppVariant.Dev]: {
    variant: AppVariant.Dev,
    name: 'OEA dev',
    title: () => 'OEA dev',
  },
  [AppVariant.Release]: {
    variant: AppVariant.Release,
    name: 'OEA',
    version: appVersion,
    title: () => t('application.title'),
  },
};

export const appPresentation: AppPresentation = presentations[appVariant];
