//! prts.json 数据加载与查询（模块级单例 ref，多个组件共享）。
//!
//! 数据来自后端 `get_prts_data` 命令，用于：
//! 1. 扫描结果卡片显示大类 / 小类的中文名（id → 中文名）；
//! 2. 自动补全输入框的候选（某子分类下所有档案标题）。
import type { PrtsData } from '@/types/prts';
import { getPrtsData } from '@/utils/tauri';
import { ref } from 'vue';

/** prts.json 完整数据（加载完成前为 null） */
export const prtsData = ref<PrtsData | null>(null);

let initialized = false;

/** 大类 id（pageType）→ 中文名；找不到时回退为原 id。 */
function getPageName(pageType: string): string {
  return prtsData.value?.PrtsPage[pageType]?.name ?? pageType;
}

/** 小类 id（categoryId）→ 中文名；找不到时回退为原 id。 */
function getCategoryName(categoryId: string): string {
  return prtsData.value?.PrtsCategory[categoryId]?.name ?? categoryId;
}

function getItemTitleById(id: string): string {
  return prtsData.value?.allItems[id]?.title ?? id;
}

/** 某子分类下所有档案标题（自动补全候选，按出现顺序去重）。 */
function getCategoryTitles(categoryId: string): string[] {
  if (!prtsData.value) {
    return [];
  }
  const titles = new Set<string>();
  for (const item of Object.values(prtsData.value.allItems)) {
    if (item.categoryId === categoryId) {
      titles.add(item.title);
    }
  }
  return [...titles];
}

export function usePrtsData() {
  if (!initialized) {
    initialized = true;

    getPrtsData().then((data) => {
      prtsData.value = data;
    });
  }

  return {
    prtsData,
    getPageName,
    getCategoryName,
    getCategoryTitles,
    getItemTitleById,
  };
}
