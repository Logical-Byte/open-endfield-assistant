/** 格式化仅影响展示，不改变业务数值与时间戳。时间沿用系统时区。 */
export const formats = {
  datetimeFormats: {
    'zh-CN': {
      date: { year: 'numeric', month: '2-digit', day: '2-digit' },
      dateTimeSeconds: {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false,
      },
      dateTime: {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false,
      },
    },
    'en-US': {
      date: { year: 'numeric', month: '2-digit', day: '2-digit' },
      dateTimeSeconds: {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false,
      },
      dateTime: {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false,
      },
    },
  },
  numberFormats: {
    'zh-CN': {
      quantity: { style: 'decimal' },
      percent: { style: 'percent', maximumFractionDigits: 1 },
    },
    'en-US': {
      quantity: { style: 'decimal' },
      percent: { style: 'percent', maximumFractionDigits: 1 },
    },
  },
} as const;
