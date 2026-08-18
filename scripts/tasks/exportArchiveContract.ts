/**
 * 生成档案库运行时契约（scripts/archive_contract.json）。
 *
 * 由 scripts/export_archive_contract.mjs 用 TypeScript 重写而来，利用
 * scripts/models 下各解包数据表模型与 src/types/archiveContract 的契约类型，
 * 将全部数据读取与档案获取方式推断逻辑类型化。
 *
 * ## 输入（ENDFIELD_DATA_DIR 指向的解包数据，全部只读）
 * - TableCfg/PrtsAllItem.json / PrtsFirstLv.json / PrtsInvestigate.json
 * - TableCfg/RewardTable.json / ShopGoodsTable.json / ShopTable.json
 * - TableCfg/SNSDialogTable.json / ReadingPopUpTable.json
 * - Json/GameplayConfig/NpcProxyTable.json / NpcProxyExDataTable.json / WorldEntityRegistry.json
 * - Json/MissionRuntimeAsset/ Json/LevelData/ Json/LevelScriptData/（递归）
 *
 * ## 输出
 * `ArchiveContract`（类型见 src/types/archiveContract.ts），由 makeAllData 写入
 * `scripts/archive_contract.json`，供 scripts/export_archive_contract.py 校验。
 */
import fs from 'node:fs';
import path from 'node:path';
import type {
  ArchiveAcquisition,
  ArchiveCategoryId,
  ArchiveContract,
  ArchiveContractRow,
  ArchiveMissionAcquisition,
  ArchiveMissionSpecial,
  ArchiveShopAcquisition,
} from '../../src/types/archiveContract';
import { endfieldDataDir, parseJSONWithBigInt } from '../gameData';
import type {
  ComponentProperties,
  LevelData,
  MissionQuest,
  MissionRuntimeAsset,
  NpcProxyExDataTable,
  NpcProxyTable,
  PrtsAllItem,
  PrtsAllItemEntry,
  PrtsFirstLv,
  PrtsInvestigate,
  ReadingPopUpTable,
  RewardTable,
  SNSDialogTable,
  ShopGoodsTable,
  ShopTable,
  WorldEntityRegistry,
} from '../models';

/** 档案分类 id（与 src/types/archiveContract 保持一致） */
const CATEGORY_IDS = [
  'paper',
  'digital',
  'collection',
  'document',
  'report',
  'media',
] as const satisfies readonly ArchiveCategoryId[];

/** 任务前缀（contentId / firstLvId 中形如 text_sm1l1m4_x 的任务 id 部分） */
const TASK_PREFIX =
  /^(?:text|radio|image|audio|video|collection|digital|document|media|paper|report)_([a-z]+\d+m\d+(?:d\d+)?|sm\d+l\d+m\d+|gm\d+m\d+|f\d+m\d+d\d+|e\d+m\d+|c\d+m\d+|a\d+m\d+)(?:_|$)/i;
/** 档案 id 前缀（nar_<task>_... 的任务 id 部分） */
const ARCHIVE_TASK_PREFIX =
  /^nar_([a-z]+\d+m\d+(?:d\d+)?|sm\d+l\d+m\d+|gm\d+m\d+|f\d+m\d+d\d+|e\d+m\d+|c\d+m\d+|a\d+m\d+)_/i;
/** 对话文本中出现的档案 id */
const NARRATIVE_ID = /nar_[A-Za-z0-9_]+/g;

/**
 * 实体没有 prts_id 组件，其 NarrativeComponent 打开该对话框，
 * 在游戏中发放指定的档案。
 */
const DIALOG_ARCHIVE_POINTS = new Map([
  ['dlg_map01_lv001_36', { archiveId: 'nar_collection_map01_10_1', pointId: 2100330123 }],
]);

/** 任务中会打开档案记录的关卡交互：关卡节点 id -> 任务文本 id */
const MISSION_INTERACTION_STAGES = new Map([
  [23100300001, 'objective_gm02m19_7_001'],
  [23100300002, 'objective_gm02m17_7_001'],
  [23100300004, 'objective_gm02m18_6_001'],
  [23100300005, 'objective_gm02m18_7_001'],
  [23100300006, 'objective_gm02m19_8_001'],
]);

/** 商店组 -> 地图区域 */
const SHOP_LOCATIONS: Record<string, { regionId: string; subregionId?: string }> = {
  domainshop_map01: { regionId: 'VL' },
  shop_common_map01_lv001_2: { regionId: 'VL', subregionId: 'HB' },
  shop_common_map01_lv003_1: { regionId: 'VL', subregionId: 'AQ' },
  shop_common_map01_lv006_1: { regionId: 'VL', subregionId: 'OL' },
  shop_common_map02_lv002_1: { regionId: 'WL', subregionId: 'WL' },
};

/** 商店表没有 shopGroupId -> NPC 代理外键，这些绑定标识普通商店的代理 */
const SHOP_PROXY_BINDINGS: Record<string, string> = {
  shop_common_map01_lv001_2: 'maji_map01_default',
  shop_common_map01_lv003_1: 'lunnade_map01_default',
  shop_common_map01_lv006_1: 'e2m6env05_map01_default',
  shop_common_map02_lv002_1: 'puyuan_map02_default',
};

/** 抛错统一前缀 */
function fail(message: string): never {
  throw new Error(`[export-archive-contract] ${message}`);
}

/** 去重并过滤空值 */
function unique<T>(values: T[]): T[] {
  return [
    ...new Set(values.filter((value) => value !== undefined && value !== null && value !== '')),
  ];
}

/** 按名称比较（稳定的排序比较器） */
function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

/** 递归列出目录下所有文件（同步） */
function filesUnder(root: string, predicate: (file: string) => boolean = () => true): string[] {
  const files: string[] = [];
  const visit = (directory: string): void => {
    const entries = fs.readdirSync(directory, { withFileTypes: true });
    entries.sort((left, right) => compareText(left.name, right.name));
    for (const entry of entries) {
      const target = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(target);
      else if (entry.isFile() && predicate(target)) files.push(target);
    }
  };
  visit(root);
  return files;
}

/** 任意 JSON 对象（脚本节点 / 关卡对象等） */
type JsonObject = Record<string, unknown>;

/** 深度遍历对象树，对每个对象调用 visitor */
function walkObjects(value: unknown, visitor: (object: JsonObject) => void): void {
  if (Array.isArray(value)) {
    for (const child of value) walkObjects(child, visitor);
    return;
  }
  if (!value || typeof value !== 'object') return;
  const object = value as JsonObject;
  visitor(object);
  for (const child of Object.values(object)) walkObjects(child, visitor);
}

/** 关卡实体组件属性表（componentProperties[组件名] 的元素） */
interface ComponentPropertyEntryShape {
  key?: string;
  value?: { valueArray?: Array<{ valueString?: string; valuestring?: string }> };
}

/**
 * 从实体的 componentProperties 提取「属性名 -> 字符串值列表」。
 * 脚本据此读取 prts_id / type_id 等组件属性。
 */
function componentStrings(entity: {
  componentProperties?: ComponentProperties;
}): Map<string, string[]> {
  const result = new Map<string, string[]>();
  for (const entries of Object.values(entity.componentProperties ?? {})) {
    for (const entry of entries as ComponentPropertyEntryShape[]) {
      if (typeof entry?.key !== 'string') continue;
      const values = (entry.value?.valueArray ?? [])
        .map((item) => item?.valueString ?? item?.valuestring)
        .filter((value): value is string => typeof value === 'string' && value.length > 0);
      if (values.length > 0) result.set(entry.key, values);
    }
  }
  return result;
}

/** 原生属性对象（{ properties: [...] } 形态，关卡脚本简表使用） */
interface RawPropertyValue {
  valueString?: string;
  valueBit64?: number;
}
interface RawProperty {
  key?: string;
  value?: { type?: string; valueArray?: RawPropertyValue[] };
}
interface RawObject {
  properties?: RawProperty[];
}

/** 读取原生属性值：字符串优先，否则取数字；多值 / StringList 返回数组 */
function propertyValue(property: RawProperty | undefined): unknown {
  const values = property?.value?.valueArray ?? [];
  if (values.length === 0) return undefined;
  const type = property?.value?.type;
  const parsed = values.map((value) =>
    value?.valueString !== undefined && value.valueString !== ''
      ? value.valueString
      : value?.valueBit64,
  );
  if (type === 'StringList' || values.length > 1) return parsed;
  return parsed[0];
}

/** 将原生属性对象转换为 key -> value 映射 */
function objectProperties(object: RawObject): Record<string, unknown> {
  return Object.fromEntries(
    (object?.properties ?? [])
      .filter(
        (property): property is RawProperty & { key: string } => typeof property?.key === 'string',
      )
      .map((property) => [property.key, propertyValue(property)]),
  );
}

/** 把 sm 档案的 _2 / _3 变体归一到 _1（逻辑档案 id） */
function logicalArchiveId(archiveId: string): string {
  const match = archiveId.match(/^(nar_sm\d+l\d+m\d+_(?:hatman|Alexander|Hans))_[23]$/);
  return match ? `${match[1]}_1` : archiveId;
}

/** 任务子任务引用 */
interface MissionQuestRef {
  missionId: string;
  questId: string;
}

/** MissionRuntimeAsset 索引 */
interface MissionIndex {
  missions: Map<string, MissionRuntimeAsset>;
  questById: Map<string, { missionId: string; quest: MissionQuest }>;
  questRefsByScriptId: Map<string, MissionQuestRef[]>;
}

/** 索引全部任务运行时资源 */
function indexMissions(): MissionIndex {
  const missionDir = path.join(endfieldDataDir, 'Json', 'MissionRuntimeAsset');
  const missionFiles = filesUnder(
    missionDir,
    (file) => file.endsWith('.json') && !file.endsWith('_meta.json'),
  );
  const missions = new Map<string, MissionRuntimeAsset>();
  const questById = new Map<string, { missionId: string; quest: MissionQuest }>();
  const questRefsByScriptId = new Map<string, MissionQuestRef[]>();

  for (const file of missionFiles) {
    const data = parseJSONWithBigInt<MissionRuntimeAsset>(fs.readFileSync(file, 'utf8'));
    const missionId = data.missionId || path.basename(file, '.json');
    missions.set(missionId, data);
    for (const [questId, quest] of Object.entries(data.questDic ?? {})) {
      questById.set(questId, { missionId, quest });
      const text = JSON.stringify(quest);
      const scriptIds = unique(
        [
          ...text.matchAll(/"scriptId"\s*:\s*(-?\d+)/g),
          ...text.matchAll(/"scriptId"\s*:\s*\{[^{}]*"scriptId"\s*:\s*(-?\d+)/g),
        ].map((match) => match[1]),
      ).filter((scriptId) => !['0', '-1'].includes(scriptId));
      for (const scriptId of scriptIds) {
        const refs = questRefsByScriptId.get(scriptId) ?? [];
        refs.push({ missionId, questId });
        questRefsByScriptId.set(scriptId, refs);
      }
    }
  }
  return { missions, questById, questRefsByScriptId };
}

/** 从档案条目推断其所属任务 id */
function taskIdFor(
  archiveId: string,
  item: PrtsAllItemEntry,
  missions: Map<string, MissionRuntimeAsset>,
): string | undefined {
  const candidates: string[] = [];
  for (const value of [item.contentId, item.firstLvId]) {
    const match = String(value ?? '').match(TASK_PREFIX);
    if (match && missions.has(match[1])) candidates.push(match[1]);
  }
  const idMatch = archiveId.match(ARCHIVE_TASK_PREFIX);
  if (idMatch && missions.has(idMatch[1])) candidates.push(idMatch[1]);
  const taskIds = unique(candidates);
  if (taskIds.length > 1)
    fail(`archive has ambiguous task prefixes: ${archiveId} (${taskIds.join(', ')})`);
  return taskIds[0];
}

/** 关卡脚本上下文（同一 scriptId 可能出现在多个关卡文件） */
interface LevelScriptContext {
  sceneId: string;
  missionHint: string | undefined;
  entityIds: number[];
}

/** 扫描 Json/LevelData 的结果 */
interface LevelDataScan {
  mapPointIds: Map<string, number>;
  levelScriptContexts: Map<string, LevelScriptContext[]>;
  missionInteractions: Map<string, number>;
}

/** 扫描全部关卡数据，收集地图点位、脚本上下文与任务交互 */
function scanLevelData(validArchiveIds: Set<string>): LevelDataScan {
  const mapPointIds = new Map<string, number>();
  const levelScriptContexts = new Map<string, LevelScriptContext[]>();
  const missionInteractions = new Map<string, number>();
  const files = filesUnder(path.join(endfieldDataDir, 'Json', 'LevelData'), (file) =>
    file.endsWith('.json'),
  );
  const missionFilePattern = /_sub_((?:sm|gm|f|e|c|a)\d+l?\d*m\d+(?:d\d+)?)/i;

  for (const file of files) {
    const data = parseJSONWithBigInt<LevelData>(fs.readFileSync(file, 'utf8'));
    const missionHint = path.basename(file, '.json').match(missionFilePattern)?.[1];
    const sceneId = data.sceneId || path.basename(path.dirname(file));

    for (const [scriptId, brief] of Object.entries(data.levelScriptBriefDataDict ?? {})) {
      const contexts = levelScriptContexts.get(scriptId) ?? [];
      contexts.push({ sceneId, missionHint, entityIds: brief.refWorldEntityIdList ?? [] });
      levelScriptContexts.set(scriptId, contexts);

      const properties = objectProperties(brief as unknown as RawObject);
      const archiveId = properties.prts as string | undefined;
      const readingPop = properties.readingPop;
      if (archiveId && validArchiveIds.has(archiveId) && readingPop) {
        const pointIds = unique(brief.refWorldEntityIdList ?? []);
        if (pointIds.length === 1) missionInteractions.set(archiveId, pointIds[0]);
      }
    }

    walkObjects(data, (object) => {
      const levelLogicId = object.levelLogicId;
      const rawComponentProperties = object.componentProperties;
      if (typeof levelLogicId !== 'number' || !rawComponentProperties) return;
      const properties = componentStrings({
        componentProperties: rawComponentProperties as ComponentProperties,
      });
      const archiveId = properties.get('prts_id')?.[0];
      if (archiveId && validArchiveIds.has(archiveId) && !mapPointIds.has(archiveId)) {
        mapPointIds.set(archiveId, levelLogicId);
      }
      const typeIds = properties.get('type_id') ?? [];
      for (const dialogId of typeIds) {
        const binding = DIALOG_ARCHIVE_POINTS.get(dialogId);
        if (!binding || levelLogicId !== binding.pointId) continue;
        mapPointIds.set(binding.archiveId, binding.pointId);
      }
    });
  }
  return { mapPointIds, levelScriptContexts, missionInteractions };
}

/** 建立「阅读弹窗 contentId -> 档案 id 列表」索引 */
function readingContentIndex(
  readingPopups: ReadingPopUpTable,
  allItems: PrtsAllItem,
): Map<string, string[]> {
  const prtsIdsByContent = new Map<string, string[]>();
  for (const [archiveId, item] of Object.entries(allItems)) {
    const ids = prtsIdsByContent.get(item.contentId) ?? [];
    ids.push(archiveId);
    prtsIdsByContent.set(item.contentId, ids);
  }
  return new Map(
    Object.entries(readingPopups).map(([readingId, reading]) => [
      readingId,
      prtsIdsByContent.get(reading.contentId) ?? [],
    ]),
  );
}

/** 从脚本对象中收集引用的任务子任务（CheckQuestState 节点） */
function questStateRefs(
  script: unknown,
  questById: Map<string, { missionId: string; quest: MissionQuest }>,
): MissionQuestRef[] {
  const refs: MissionQuestRef[] = [];
  walkObjects(script, (object) => {
    if (!String(object['$type'] ?? '').includes('CheckQuestState')) return;
    const questIdValue = (object['_questId'] as { constValue?: unknown } | undefined)?.constValue;
    const quest = questById.get(String(questIdValue));
    if (quest) refs.push({ missionId: quest.missionId, questId: String(questIdValue) });
  });
  return refs;
}

/** 从脚本对象中收集已知的阅读弹窗 id（ShowUIReadingPopPanel 节点） */
function readingIdsInScript(script: unknown, knownReadingIds: Set<string>): string[] {
  const ids = new Set<string>();
  walkObjects(script, (object) => {
    if (!String(object['$type'] ?? '').includes('ShowUIReadingPopPanel')) return;
    const readingId = (object['_readingPopId'] as { constValue?: unknown } | undefined)?.constValue;
    if (typeof readingId === 'string' && knownReadingIds.has(readingId)) ids.add(readingId);
  });
  const text = JSON.stringify(script);
  for (const match of text.matchAll(/"valueString"\s*:\s*"([^"]+)"/g)) {
    if (knownReadingIds.has(match[1])) ids.add(match[1]);
  }
  return [...ids];
}

/** 扫描 Json/LevelScriptData，推断档案所属任务子任务 */
function scanLevelScripts({
  readingPopups,
  allItems,
  levelScriptContexts,
  questById,
  questRefsByScriptId,
}: {
  readingPopups: ReadingPopUpTable;
  allItems: PrtsAllItem;
  levelScriptContexts: Map<string, LevelScriptContext[]>;
  questById: Map<string, { missionId: string; quest: MissionQuest }>;
  questRefsByScriptId: Map<string, MissionQuestRef[]>;
}): Map<string, string> {
  const archiveQuestIds = new Map<string, string>();
  const readingToArchives = readingContentIndex(readingPopups, allItems);
  const knownReadingIds = new Set(readingToArchives.keys());
  const files = filesUnder(path.join(endfieldDataDir, 'Json', 'LevelScriptData'), (file) =>
    file.endsWith('.json'),
  );

  for (const file of files) {
    const script = parseJSONWithBigInt<unknown>(fs.readFileSync(file, 'utf8'));
    const scriptId = path.basename(file, '.json');
    const archiveIds = unique(
      readingIdsInScript(script, knownReadingIds).flatMap(
        (readingId) => readingToArchives.get(readingId) ?? [],
      ),
    );
    if (archiveIds.length === 0) continue;

    const contexts = levelScriptContexts.get(scriptId) ?? [];
    const refs = unique(
      [...(questRefsByScriptId.get(scriptId) ?? []), ...questStateRefs(script, questById)].map(
        (ref) => `${ref.missionId}\u0000${ref.questId}`,
      ),
    ).map((ref) => {
      const [missionId, questId] = ref.split('\u0000');
      return { missionId, questId };
    });
    const missionHints = unique(contexts.map((context) => context.missionHint));

    for (const archiveId of archiveIds) {
      const taskIds = unique([...refs.map((ref) => ref.missionId), ...missionHints]);
      const relevantRefs = refs.filter((ref) => taskIds.includes(ref.missionId));
      if (relevantRefs.length === 1) archiveQuestIds.set(archiveId, relevantRefs[0].questId);
    }
  }
  return archiveQuestIds;
}

/** 扫描 SNS 对话，推断档案所属任务子任务 */
function scanSnsQuestIds(
  snsDialogs: SNSDialogTable,
  missions: Map<string, MissionRuntimeAsset>,
  allItems: PrtsAllItem,
  taskIdsByArchive: Map<string, string>,
): Map<string, string> {
  const archiveQuestIds = new Map<string, string>();
  const archivesByDialog = new Map<string, string[]>();
  const archivesByChat = new Map<string, string[]>();
  for (const [dialogId, dialog] of Object.entries(snsDialogs)) {
    if (dialogId === 'sns_test_prts') continue;
    const archiveIds = unique(JSON.stringify(dialog).match(NARRATIVE_ID) ?? []).filter(
      (archiveId) => allItems[archiveId],
    );
    archivesByDialog.set(dialogId, archiveIds);
    if (dialog.chatId) {
      const ids = archivesByChat.get(dialog.chatId) ?? [];
      ids.push(...archiveIds);
      archivesByChat.set(dialog.chatId, unique(ids));
    }
  }

  for (const [missionId, mission] of missions) {
    if (missionId === 'hidden8') continue;
    for (const [questId, quest] of Object.entries(mission.questDic ?? {})) {
      const text = JSON.stringify(quest);
      const dialogIds = unique(
        [
          ...text.matchAll(/"snsDialogId"\s*:\s*"([^"]+)"/g),
          ...text.matchAll(/"_dialogId"\s*:\s*\{\s*"constValue"\s*:\s*"([^"]+)"/g),
        ].map((match) => match[1]),
      );
      for (const dialogId of dialogIds) {
        if (dialogId === 'sns_test_prts') continue;
        const directArchiveIds = archivesByDialog.get(dialogId) ?? [];
        const chatId = snsDialogs[dialogId]?.chatId;
        const archiveIds =
          directArchiveIds.length > 0
            ? directArchiveIds
            : (archivesByChat.get(chatId) ?? []).filter(
                (archiveId) => taskIdsByArchive.get(archiveId) === missionId,
              );
        for (const archiveId of archiveIds) archiveQuestIds.set(archiveId, questId);
      }
    }
  }
  return archiveQuestIds;
}

/** 建立「研究最终解锁档案 -> researchId」索引 */
function researchIndex(investigations: PrtsInvestigate): Map<string, string> {
  const result = new Map<string, string>();
  for (const [researchId, research] of Object.entries(investigations)) {
    if (!research.unlockPrts) continue;
    if (result.has(research.unlockPrts))
      fail(`archive has multiple unlock investigations: ${research.unlockPrts}`);
    result.set(research.unlockPrts, researchId);
  }
  return result;
}

/** 建立「奖励表 -> 档案 id 列表」索引 */
function rewardArchiveIndex(
  rewards: RewardTable,
  validArchiveIds: Set<string>,
): Map<string, string[]> {
  const result = new Map<string, string[]>();
  for (const [rewardId, reward] of Object.entries(rewards)) {
    const archives = unique(
      (['itemBundles', 'probItemBundles'] as const).flatMap((field) =>
        (reward[field] ?? []).map((bundle) => bundle.id).filter((id) => validArchiveIds.has(id)),
      ),
    );
    if (archives.length > 0) result.set(rewardId, archives);
  }
  return result;
}

/** 建立「档案 -> 商店获取信息」索引 */
function shopIndex({
  rewards,
  shopGoods,
  shops,
  validArchiveIds,
}: {
  rewards: RewardTable;
  shopGoods: ShopGoodsTable;
  shops: ShopTable;
  validArchiveIds: Set<string>;
}): Map<string, ArchiveShopAcquisition> {
  const archivesByReward = rewardArchiveIndex(rewards, validArchiveIds);
  const result = new Map<string, ArchiveShopAcquisition>();
  for (const goods of Object.values(shopGoods)) {
    const archiveIds = archivesByReward.get(goods.rewardId) ?? [];
    const shop = shops[goods.shopId];
    if (archiveIds.length === 0 || !shop) continue;
    for (const archiveId of archiveIds) {
      const acquisition: ArchiveShopAcquisition = {
        method: 'shop',
        shopGroupId: shop.shopGroupId,
        shopId: goods.shopId,
      };
      if (!SHOP_LOCATIONS[shop.shopGroupId]) fail(`missing shop location for ${shop.shopGroupId}`);
      result.set(archiveId, acquisition);
    }
  }
  return result;
}

/** 为商店获取信息补充 NPC / 点位 / 位置 */
function attachShopNpcPoints({
  shopAcquisitions,
  npcProxies,
  npcProxyExtra,
  worldEntities,
}: {
  shopAcquisitions: Map<string, ArchiveShopAcquisition>;
  npcProxies: NpcProxyTable;
  npcProxyExtra: NpcProxyExDataTable;
  worldEntities: WorldEntityRegistry;
}): void {
  const registryByProxy = new Map(
    Object.values(worldEntities.npcProxyBriefInfos ?? {}).map(
      (entry) => [entry.proxyId, entry] as const,
    ),
  );
  const proxyTable = npcProxies.dataTable ?? {};
  for (const acquisition of shopAcquisitions.values()) {
    const proxyId = SHOP_PROXY_BINDINGS[acquisition.shopGroupId];
    if (!proxyId) continue;
    const proxy = proxyTable[proxyId];
    const proxyInfo = npcProxyExtra.proxyInfoData?.[proxyId];
    const registry = registryByProxy.get(proxyId);
    if (!proxy || !proxyInfo || !registry) fail(`missing bound shop proxy data: ${proxyId}`);
    acquisition.npcId = proxyInfo.npcId;
    acquisition.pointId = registry.segmentIdGlobal;
  }
  for (const acquisition of shopAcquisitions.values()) {
    acquisition.location = SHOP_LOCATIONS[acquisition.shopGroupId];
  }
}

/** nar_002_settlement 的特殊信息（NPC 对话 + Baker 消息链） */
function missionSpecial(
  archiveId: string,
  snsDialogs: SNSDialogTable,
): ArchiveMissionSpecial | undefined {
  if (archiveId !== 'nar_002_settlement') return undefined;
  const bakerDialog = snsDialogs.sns_f1m4d1_4;
  if (!bakerDialog || bakerDialog.chatId !== 'sns_chat_nfm_0_1') {
    fail('missing Baker dialog for nar_002_settlement');
  }
  return {
    npcId: 'suosi_map01',
    dialogOptionId: 'option_dlg_map01_lv002_env_8_1_001',
    bakerChatId: bakerDialog.chatId,
    bakerDialogId: bakerDialog.dialogId,
  };
}

/** 综合各索引，判定单个档案的获取方式 */
function archiveAcquisition({
  archiveId,
  item,
  mapPointIds,
  researchIds,
  shopAcquisitions,
  missions,
  questIds,
  missionInteractions,
  snsDialogs,
}: {
  archiveId: string;
  item: PrtsAllItemEntry;
  mapPointIds: Map<string, number>;
  researchIds: Map<string, string>;
  shopAcquisitions: Map<string, ArchiveShopAcquisition>;
  missions: Map<string, MissionRuntimeAsset>;
  questIds: Map<string, string>;
  missionInteractions: Map<string, number>;
  snsDialogs: SNSDialogTable;
}): ArchiveAcquisition {
  const mapPointId = mapPointIds.get(archiveId);
  if (mapPointId !== undefined) return { method: 'map', pointId: mapPointId };

  const researchId = researchIds.get(archiveId);
  if (researchId) return { method: 'invstgt', researchId };

  const taskId =
    taskIdFor(archiveId, item, missions) ??
    (archiveId === 'nar_002_settlement' ? 'f1m4d1' : undefined);
  if (taskId) {
    const acquisition: ArchiveMissionAcquisition = { method: 'mission', missionId: taskId };
    const questId =
      questIds.get(archiveId) ?? (archiveId === 'nar_002_settlement' ? 'f1m4d1_q#11' : undefined);
    if (questId) acquisition.questId = questId;
    const pointId = missionInteractions.get(archiveId);
    if (pointId !== undefined) {
      const stageId = MISSION_INTERACTION_STAGES.get(pointId);
      if (!stageId) fail(`missing mission interaction stage for ${archiveId}: ${pointId}`);
      acquisition.interaction = { pointId, stageId };
    }
    const special = missionSpecial(archiveId, snsDialogs);
    if (special) acquisition.special = special;
    return acquisition;
  }

  return shopAcquisitions.get(archiveId) ?? { method: 'auto' };
}

/** 构建最终契约（分组、归并逻辑 id、排序） */
function buildContract({
  allItems,
  firstLevels,
  mapPointIds,
  researchIds,
  shopAcquisitions,
  missions,
  questIds,
  missionInteractions,
  snsDialogs,
}: {
  allItems: PrtsAllItem;
  firstLevels: PrtsFirstLv;
  mapPointIds: Map<string, number>;
  researchIds: Map<string, string>;
  shopAcquisitions: Map<string, ArchiveShopAcquisition>;
  missions: Map<string, MissionRuntimeAsset>;
  questIds: Map<string, string>;
  missionInteractions: Map<string, number>;
  snsDialogs: SNSDialogTable;
}): ArchiveContract {
  const groups = new Map<string, string[]>();
  for (const archiveId of Object.keys(allItems)) {
    const logicalId = logicalArchiveId(archiveId);
    const ids = groups.get(logicalId) ?? [];
    ids.push(archiveId);
    groups.set(logicalId, ids);
  }

  const rows: Array<{
    id: string;
    categoryId: ArchiveCategoryId;
    icon: string;
    acquisition: ArchiveAcquisition;
    order: [number, number, string];
  }> = [];
  for (const [archiveId, sourceIds] of groups) {
    const representativeId = sourceIds.includes(archiveId) ? archiveId : sourceIds[0];
    const item = allItems[representativeId];
    const firstLevel = firstLevels[item.firstLvId];
    if (!firstLevel) fail(`missing PrtsFirstLv row for ${representativeId}: ${item.firstLvId}`);
    if (!CATEGORY_IDS.includes(firstLevel.categoryId as ArchiveCategoryId)) {
      fail(`unknown archive category for ${representativeId}: ${firstLevel.categoryId}`);
    }
    rows.push({
      id: archiveId,
      categoryId: firstLevel.categoryId as ArchiveCategoryId,
      icon: firstLevel.icon,
      acquisition: archiveAcquisition({
        archiveId: representativeId,
        item,
        mapPointIds,
        researchIds,
        shopAcquisitions,
        missions,
        questIds,
        missionInteractions,
        snsDialogs,
      }),
      order: [Number(firstLevel.order ?? 0), Number(item.order ?? 0), archiveId],
    });
  }

  rows.sort(
    (left, right) =>
      CATEGORY_IDS.indexOf(left.categoryId) - CATEGORY_IDS.indexOf(right.categoryId) ||
      left.order[0] - right.order[0] ||
      left.order[1] - right.order[1] ||
      compareText(left.order[2], right.order[2]),
  );

  return {
    version: 1,
    categories: Object.fromEntries(
      CATEGORY_IDS.map((categoryId) => [
        categoryId,
        rows
          .filter((row) => row.categoryId === categoryId)
          .map(({ id, icon, acquisition }): ArchiveContractRow => ({ id, icon, acquisition })),
      ]),
    ) as Record<ArchiveCategoryId, ArchiveContractRow[]>,
  };
}

/**
 * 生成档案库运行时契约。
 * 由 makeAllData 调用并写入 scripts/archive_contract.json。
 */
export function exportArchiveContract(): ArchiveContract {
  const read = <T>(relativePath: string): T =>
    parseJSONWithBigInt<T>(fs.readFileSync(path.join(endfieldDataDir, relativePath), 'utf8'));

  const allItems = read<PrtsAllItem>('TableCfg/PrtsAllItem.json');
  const firstLevels = read<PrtsFirstLv>('TableCfg/PrtsFirstLv.json');
  const investigations = read<PrtsInvestigate>('TableCfg/PrtsInvestigate.json');
  const rewards = read<RewardTable>('TableCfg/RewardTable.json');
  const shopGoods = read<ShopGoodsTable>('TableCfg/ShopGoodsTable.json');
  const shops = read<ShopTable>('TableCfg/ShopTable.json');
  const snsDialogs = read<SNSDialogTable>('TableCfg/SNSDialogTable.json');
  const readingPopups = read<ReadingPopUpTable>('TableCfg/ReadingPopUpTable.json');
  const npcProxies = read<NpcProxyTable>('Json/GameplayConfig/NpcProxyTable.json');
  const npcProxyExtra = read<NpcProxyExDataTable>('Json/GameplayConfig/NpcProxyExDataTable.json');
  const worldEntities = read<WorldEntityRegistry>('Json/GameplayConfig/WorldEntityRegistry.json');
  const missionIndex = indexMissions();

  const validArchiveIds = new Set(Object.keys(allItems));
  const taskIdsByArchive = new Map<string, string>(
    Object.entries(allItems)
      .map(([archiveId, item]) => [archiveId, taskIdFor(archiveId, item, missionIndex.missions)])
      .filter((entry): entry is [string, string] => entry[1] !== undefined),
  );
  const { mapPointIds, levelScriptContexts, missionInteractions } = scanLevelData(validArchiveIds);
  const levelScriptQuestIds = scanLevelScripts({
    readingPopups,
    allItems,
    levelScriptContexts,
    questById: missionIndex.questById,
    questRefsByScriptId: missionIndex.questRefsByScriptId,
  });
  const snsQuestIds = scanSnsQuestIds(
    snsDialogs,
    missionIndex.missions,
    allItems,
    taskIdsByArchive,
  );
  const questIds = new Map<string, string>([...levelScriptQuestIds, ...snsQuestIds]);
  const researchIds = researchIndex(investigations);
  const shopAcquisitions = shopIndex({ rewards, shopGoods, shops, validArchiveIds });
  attachShopNpcPoints({ shopAcquisitions, npcProxies, npcProxyExtra, worldEntities });

  const contract = buildContract({
    allItems,
    firstLevels,
    mapPointIds,
    researchIds,
    shopAcquisitions,
    missions: missionIndex.missions,
    questIds,
    missionInteractions,
    snsDialogs,
  });

  const rows = Object.values(contract.categories).flat();
  const methodCounts = Object.fromEntries(
    (['map', 'mission', 'auto', 'shop', 'invstgt'] as const).map((method) => [
      method,
      rows.filter((row) => row.acquisition.method === method).length,
    ]),
  );
  console.log(`[exportArchiveContract] ${rows.length} archives ${JSON.stringify(methodCounts)}`);
  return contract;
}
