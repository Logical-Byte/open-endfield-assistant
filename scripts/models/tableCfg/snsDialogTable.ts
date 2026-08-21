import type { TranslationKey } from '../common';

/** SNS 对话中的单条内容节点 */
export interface SNSDialogContent {
  content: TranslationKey;
  contentId: number;
  contentParam: unknown[];
  contentParams: string;
  contentType: number;
  dialogOptionIds: unknown[];
  /** 是否为对话结尾 */
  isEnd: boolean;
  linkMissionId: string;
  linkRewardId: string;
  nextContentId: number;
  optionType: number;
  preContentId: number;
  /** 说话人 id（如 sns_chr_0004_pelica） */
  speaker: string;
}

/** SNSDialogTable.json 中的单个对话 */
export interface SNSDialog {
  /** 所属消息链 id（如 sns_chr_0004_pelica） */
  chatId: string;
  /** 内容节点表：contentId -> 节点 */
  dialogContentData: Record<string, SNSDialogContent>;
  /** 对话 id */
  dialogId: string;
  dialogType: number;
  noticeType: number;
  /** 关联任务 id */
  relatedMissionId: string;
  skipToFirstOption: boolean;
  topicId: string;
}

/** SNSDialogTable.json：dialogId -> 对话 */
export type SNSDialogTable = Record<string, SNSDialog>;
