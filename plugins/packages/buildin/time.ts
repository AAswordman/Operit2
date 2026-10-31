/* METADATA
{
  name: time

  display_name: {
    zh: "时间"
    en: "Time"
  }
  description: {
    zh: "提供时间相关功能。实际上，激活本包的同时已经能够获取时间了。"
    en: "Provides time-related utilities. In practice, current time is already available once this package is enabled."
  }
  enabledByDefault: true
  category: "Utility"
  tools: [
    {
      name: get_time
      display: {"label": {"zh": "获取当前时间", "en": "Get current time"}, "icon": "time"}
      description: {
        zh: "获取当前时间。当使用此包时，AI已经自动获取了当前的时间信息。"
        en: "Get the current time. When using this package, the AI may already have the current time context."
      }
      parameters: []
    },
    {
      name: format_time
      display: {"label": {"zh": "格式化时间", "en": "Format time"}, "icon": "time"}
      description: {
        zh: "格式化时间。提供各种时间格式化选项。"
        en: "Format time. Provides various time formatting options."
      }
      parameters: []
    }
  ]
}*/

const timePackage = (function () {
  async function get_time(): Promise<any> {
    const now = new Date();

    return {
      timestamp: now.getTime(),
      iso: now.toISOString(),
      local: now.toLocaleString(),
      date: {
        year: now.getFullYear(),
        month: now.getMonth() + 1,
        day: now.getDate(),
        weekday: now.toLocaleDateString(undefined, { weekday: 'long' })
      },
      time: {
        hours: now.getHours(),
        minutes: now.getMinutes(),
        seconds: now.getSeconds()
      }
    };
  }

  async function format_time(): Promise<any> {
    const now = new Date();

    const pad = (n: number) => n.toString().padStart(2, '0');

    const hours = now.getHours();
    const minutes = now.getMinutes();
    const seconds = now.getSeconds();

    const time24h = `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;

    const suffix = hours >= 12 ? 'PM' : 'AM';
    const hour12 = hours % 12 === 0 ? 12 : hours % 12;
    const time12h = `${pad(hour12)}:${pad(minutes)}:${pad(seconds)} ${suffix}`;

    return {
      iso: now.toISOString(),
      date: `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`,
      time24h,
      time12h
    };
  }

  return {
    get_time,
    format_time
  };
})();

// 逐个导出
exports.get_time = timePackage.get_time;
exports.format_time = timePackage.format_time;