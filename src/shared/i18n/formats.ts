/** 格式化仅影响展示，不改变业务数值与时间戳。时间沿用系统时区。 */
export const dateFormats = {
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
} satisfies Record<string, Intl.DateTimeFormatOptions>;

export const numberFormats = {
  quantity: { style: 'decimal' },
  percent: { style: 'percent', maximumFractionDigits: 1 },
} satisfies Record<string, Intl.NumberFormatOptions>;
