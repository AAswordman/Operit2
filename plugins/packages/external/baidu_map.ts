/*
METADATA
{
    "api_version": "2.0.0",
    "name": "baidu_map",
    "display_name": {
        "zh": "百度地图工具",
        "en": "Baidu Map Tools"
    },
    "description": { "zh": "百度地图工具集合，提供AOI（兴趣区域）数据获取接口。通过调用百度地图API，支持按地理范围查询AOI边界坐标，基于位置的路线规划，助力地理信息系统应用开发和空间数据分析。", "en": "A Baidu Maps toolkit that provides AOI (Area of Interest) data access. It supports querying AOI boundary coordinates by geographic range and location-based route planning, useful for GIS development and spatial data analysis." },
    "enabledByDefault": true,
    "category": "Map",
    "tools": [
        {
            "name": "search_aoi",
            "description": { "zh": "搜索百度地图兴趣区域(AOI)信息", "en": "Search AOI (Area of Interest) information from Baidu Maps." },
            "parameters": [
                {
                    "name": "keyword",
                    "description": { "zh": "搜索关键词，如商场、小区名称等", "en": "Search keyword, e.g. mall name, residential community name, etc." },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "city_name",
                    "description": { "zh": "城市名称，如'北京'，默认全国范围", "en": "City name, e.g. '北京'. Defaults to nationwide." },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "planRoute",
            "description": { "zh": "智能路线规划，从当前位置到指定目的地，并发返回驾车、步行、公交三种方式的路线规划。", "en": "Smart route planning from current location to a destination, returning driving/walking/transit plans in parallel." },
            "parameters": [
                {
                    "name": "destination",
                    "description": { "zh": "目的地名称", "en": "Destination name." },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "city_name",
                    "description": { "zh": "城市名称，辅助目的地查找", "en": "City name to help resolve the destination (optional)." },
                    "type": "string",
                    "required": false
                }
            ]
        },
    ]
}*/

// Baidu Map API Type Definitions
namespace BaiduMap {
    export interface Point {
        x: string;
        y: string;
    }

    export interface TicketInfo {
        title?: string;
        price?: string;
        marketprice?: string;
        sold_info?: string;
        bookingTimeTag?: string[];
        ticket_type_name?: string;
    }

    export interface RankingInfo {
        ranking?: string;
        rank_s?: string;
        score?: number;
        short_name?: string;
        ranking_type?: string;
    }

    export interface EventNotice {
        title?: string;
        content?: string;
        img_url?: string;
        start_end?: string;
    }

    export interface BangdanHead {
        ranking_show?: RankingInfo[];
    }

    export interface DetailInfo {
        point?: Point;
        guoke_geo?: {
            geo: string;
        };
        tag?: string;
        overall_rating?: string;
        comment_num?: string;
        price?: string;
        shop_hours_simple?: string;
        photo_num?: string;
        indoor_map?: string;
        aoi_src_id?: string;
        navi_update_time?: string;
        official_url?: string;
        is_reservable?: string;
        dk_ticket?: TicketInfo;
        bangdan_head?: BangdanHead;
        event_notice?: EventNotice;
        areaid?: string;
        entrance_price?: string;
        free?: string;
    }

    export interface Ext {
        detail_info?: DetailInfo;
        geo?: string;
    }

    export interface BusinessTimeInfo {
        common?: string;
        festival?: string;
    }

    export interface BusinessTimeData {
        business_time_text?: BusinessTimeInfo;
        common?: BusinessTimeSchedule[];
        festival?: Record<string, BusinessTimeSchedule[]>;
    }

    export interface BusinessTimeSchedule {
        date?: string;
        season?: string;
        special?: string;
        weeks?: Record<string, TimeSlot[]>;
    }

    export interface TimeSlot {
        time?: string;
        type?: string;
    }

    export interface BusinessTime {
        data?: BusinessTimeData[];
    }

    export interface ContentItem {
        uid?: string;
        name?: string;
        alias?: string;
        addr?: string;
        area_name?: string;
        tel?: string;
        cla?: [number, string][];
        ext?: Ext;
        geo?: string;
        geodata?: unknown;
        guoke_geo?: { geo: string };
        geo_type?: number;
        x?: string;
        y?: string;
        point?: Point;
        di_tag?: string;
        std_tag?: string;
        overall_rating?: string;
        business_time?: BusinessTime;
        shop_hours_simple?: string;
        pano?: string | number;
        area_code?: string | number;
        city_id?: string | number;
        [key: string]: unknown;
    }

    export interface ApiResponse {
        content?: ContentItem[];
        current_city?: {
            code?: string | number;
        };
        result?: {
            code?: string | number;
            city_id?: string | number;
        };
        [key: string]: unknown;
    }
}

/**
 * 门票信息接口
 */
interface TicketInfoResult {
    title?: string;
    price?: string;
    market_price?: string;
    sold_info?: string;
    booking_tags: string[];
    ticket_type?: string;
}

/**
 * 排行榜信息接口
 */
interface RankingResult {
    name?: string;
    rank?: string;
    score?: number;
    short_name?: string;
    type?: string;
}

/**
 * 活动事件接口
 */
interface EventResult {
    title?: string;
    content?: string;
    img_url?: string;
    start_end?: string;
}

/**
 * 详细开放时间接口
 */
interface OpeningHoursDetail {
    common_hours?: string;
    festival_hours?: string;
    detailed_schedule: BaiduMap.BusinessTimeSchedule[];
    festival_schedule: Record<string, BaiduMap.BusinessTimeSchedule[]>;
}

/**
 * AOI (Area of Interest) item structure returned from search.
 */
interface AoiDataItem {
    uid: string;
    name: string;
    address: string;
    area_name?: string;
    phone?: string;
    tags?: string;
    detail_type?: string;
    rating?: number;
    comment_count?: number;
    price?: string;
    ticket_info?: TicketInfoResult;
    opening_hours?: string;
    opening_hours_detail?: OpeningHoursDetail;
    shop_hours_simple?: string;
    photo_count?: number;
    has_street_view?: boolean;
    street_view_info?: { pid: string; heading: number };
    has_indoor_map?: boolean;
    rankings: RankingResult[];
    events: EventResult[];
    type: string;
    has_geo_data: boolean;
    center: {
        lng: number;
        lat: number;
    };
    detail_url: string;
    additional_info: Record<string, unknown>;
    raw_data?: Partial<BaiduMap.ContentItem>;
}

/**
 * Result structure for the `search_aoi` function.
 */
interface SearchAoiResult {
    success: boolean;
    message?: string;
    keyword: string;
    city_name?: string;
    total: number;
    aois: AoiDataItem[];
    boundary?: AoiBoundaryResult;
}

/**
 * Result structure for the `get_aoi_boundary` function.
 */
interface AoiBoundaryResult {
    success: boolean;
    message?: string;
    uid: string;
    name?: string;
    address?: string;
    center?: {
        lng: number;
        lat: number;
    };
    boundary?: { lng: number, lat: number }[];
    point_count?: number;
    raw_data?: Partial<BaiduMap.ContentItem>;
}

/**
 * Location information structure.
 */
interface LocationInfo {
    lat: number;
    lng: number;
    address?: string;
}

/**
 * Route details for a specific transport mode.
 */
interface RouteDetails {
    estimated_distance: string;
    estimated_duration: string;
    transport_mode: "driving" | "walking" | "transit";
    navigation_url: string;
    suggestion: string;
}

/**
 * Destination information structure.
 */
interface DestinationInfo {
    name: string;
    address: string;
    location: LocationInfo;
}

/**
 * Route planning result structure.
 */
interface RouteResult {
    success: boolean;
    message?: string;
    current_location?: LocationInfo;
    destination?: DestinationInfo;
    all_routes?: {
        driving: RouteDetails;
        walking: RouteDetails;
        transit: RouteDetails;
    };
}

/**
 * Test results structure.
 */
interface BaiduMapTestResults {
    aoi_search?: {
        success: boolean;
        total?: number;
        error?: string;
    };
    advanced_feature?: {
        success: boolean;
        uid?: string;
        point_count?: number;
        error?: string;
    };
    route_planning?: {
        success: boolean;
        destination?: string;
        distance?: string;
        duration?: string;
        error?: string;
    };
    route_planning_all_modes?: {
        success: boolean;
        destination?: string;
        routes?: {
            driving: RouteDetails;
            walking: RouteDetails;
            transit: RouteDetails;
        };
        error?: string;
    };
}


const baiduMap = (function () {
    // 常用城市编码
    const CITY_CODES = {
        "北京": "131",
        "上海": "289",
        "广州": "257",
        "深圳": "340",
        "杭州": "179",
        "南京": "315",
        "武汉": "218",
        "成都": "75",
        "重庆": "132",
        "西安": "233",
        "全国": "1" // 默认值
    };

    // 请求头配置
    const HEADERS = {
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7',
        'Accept-Language': 'zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6',
        'Cache-Control': 'no-cache',
        'Connection': 'keep-alive',
        'Host': 'map.baidu.com',
        'Pragma': 'no-cache',
        'Sec-Fetch-Dest': 'document',
        'Sec-Fetch-Mode': 'navigate',
        'Sec-Fetch-Site': 'none',
        'Sec-Fetch-User': '?1',
        'Upgrade-Insecure-Requests': '1',
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0',
        'sec-ch-ua': '"Chromium";v="136", "Microsoft Edge";v="136", "Not.A/Brand";v="99"',
        'sec-ch-ua-mobile': '?0',
        'sec-ch-ua-platform': '"Windows"',
        'Referer': 'https://map.baidu.com/'
    };

    // 日志级别配置
    const LOG_LEVELS = {
        NONE: 0,    // 不输出任何日志
        ERROR: 1,   // 只输出错误信息
        WARN: 2,    // 输出警告和错误
        INFO: 3,    // 输出普通信息、警告和错误
        DEBUG: 4,   // 输出调试信息、普通信息、警告和错误
        TRACE: 5    // 输出所有日志，包括跟踪信息
    };

    // 默认日志级别
    let currentLogLevel = LOG_LEVELS.INFO;

    /**
     * 设置日志级别
     */
    function setLogLevel(level: number): void {
        if (level >= LOG_LEVELS.NONE && level <= LOG_LEVELS.TRACE) {
            currentLogLevel = level;
        }
    }

    /**
     * 统一的日志输出函数
     */
    function logger(level: number, message: string, data?: unknown): void {
        if (level > currentLogLevel) return;

        const levelNames = {
            [LOG_LEVELS.ERROR]: '[ERROR]',
            [LOG_LEVELS.WARN]: '[WARN]',
            [LOG_LEVELS.INFO]: '[INFO]',
            [LOG_LEVELS.DEBUG]: '[DEBUG]',
            [LOG_LEVELS.TRACE]: '[TRACE]'
        };

        const levelName = levelNames[level] || '[UNKNOWN]';
        const timestamp = new Date().toISOString();
        const logMessage = `${timestamp} ${levelName} ${message}`;

        console.log(logMessage);
    }

    /**
     * 对中文进行URL编码
     */
    function encodeURIComponentSafe(str: string): string {
        try {
            return encodeURIComponent(str);
        } catch (e) {
            logger(LOG_LEVELS.ERROR, `编码失败:`, e);
            return str;
        }
    }

    /**
     * 使用OkHttp发起GET请求
     */
    async function httpGet(url: string): Promise<any> {
        try {
            const client = createHttpClient();
            const response = await client.get(url, HEADERS);

            if (!response.isSuccessful()) {
                throw new Error(`请求失败: ${response.statusCode} ${response.statusMessage}`);
            }

            try {
                const jsonResponse = response.json();
                return jsonResponse;
            } catch (e) {
                logger(LOG_LEVELS.ERROR, `解析JSON失败:`, e);
                return response.content;
            }
        } catch (e) {
            logger(LOG_LEVELS.ERROR, `网络请求错误:`, e);
            throw e;
        }
    }

    /**
     * 搜索百度地图AOI信息
     * 参考 https://blog.csdn.net/Jacey_cai/article/details/131524758
     */
    async function search_aoi(params: {
        keyword: string;
        city_name?: string;  // 城市名，如"北京"
    }): Promise<SearchAoiResult> {
        try {
            // 参数处理
            const keyword = params.keyword;
            if (!keyword) {
                throw new Error("关键词不能为空");
            }

            // 处理城市编码 - 改进版
            let cityCode = "1"; // 默认全国

            if (params.city_name) {
                // 使用动态获取城市编码功能
                cityCode = await getCityCode(params.city_name);
                logger(LOG_LEVELS.DEBUG, `城市 "${params.city_name}" 对应的编码:`, cityCode);
            }

            // 构建URL - 使用搜索API
            const encodedKeyword = encodeURIComponentSafe(keyword);
            const url = `https://map.baidu.com/?newmap=1&qt=s&da_src=searchBox.button&wd=${encodedKeyword}&c=${cityCode}`;

            logger(LOG_LEVELS.INFO, `搜索AOI: ${keyword}, 城市名称: ${params.city_name || '全国'}`);

            // 发起请求
            const result = await httpGet(url) as BaiduMap.ApiResponse;

            // 根据实际结构，直接使用content数组
            const dataContent = result?.content || [];

            // 如果没有找到任何内容数据结构，返回空结果
            if (!dataContent || dataContent.length === 0) {
                logger(LOG_LEVELS.INFO, `搜索结果为空或格式不符合预期`);
                return {
                    success: true,
                    keyword: keyword,
                    city_name: params.city_name,
                    total: 0,
                    aois: [] as AoiDataItem[]
                };
            }

            // 解析content数组中的每个元素
            const potentialAois: AoiDataItem[] = dataContent.map((item: BaiduMap.ContentItem) => {
                // ==========基本信息==========
                const uid = item.uid || "";
                const name = item.name || (item.alias) || "";
                const address = item.addr || "";
                const area_name = item.area_name;
                const phone = item.tel;

                // ==========分类信息==========
                let type = "";
                if (item.cla && Array.isArray(item.cla) && item.cla.length > 0) {
                    type = item.cla.map(cla => cla[1]).join(", ");
                }
                const detail_type = item.std_tag;
                const tags = item.di_tag;

                // ==========评分和评论==========
                let rating: number | undefined = undefined;
                let comment_count: number | undefined = undefined;

                // 从多个位置提取评分
                const overallRating = item.overall_rating || item.ext?.detail_info?.overall_rating;
                if (overallRating) {
                    rating = parseFloat(overallRating);
                }

                // 从多个位置提取评论数
                const commentNum = item.ext?.detail_info?.comment_num;
                if (commentNum) {
                    comment_count = parseInt(commentNum, 10);
                }

                // ==========价格信息==========
                let price: string | undefined = undefined;
                let ticket_info: TicketInfoResult | undefined = undefined;

                // 从detail_info提取价格
                if (item.ext?.detail_info?.price) {
                    price = item.ext.detail_info.price;
                }

                // 提取门票信息
                if (item.ext?.detail_info?.dk_ticket) {
                    const ticketData = item.ext.detail_info.dk_ticket;
                    ticket_info = {
                        title: ticketData.title,
                        price: ticketData.price,
                        market_price: ticketData.marketprice,
                        sold_info: ticketData.sold_info,
                        booking_tags: ticketData.bookingTimeTag || [],
                        ticket_type: ticketData.ticket_type_name
                    };
                }

                // ==========开放时间==========
                let opening_hours: string | undefined = undefined;
                let opening_hours_detail: OpeningHoursDetail | undefined = undefined;

                if (item.business_time?.data?.[0]) {
                    const timeData = item.business_time.data[0];
                    opening_hours = timeData.business_time_text?.common;

                    // 详细开放时间信息
                    opening_hours_detail = {
                        common_hours: timeData.business_time_text?.common,
                        festival_hours: timeData.business_time_text?.festival,
                        detailed_schedule: timeData.common || [],
                        festival_schedule: timeData.festival || {}
                    };
                }

                // ==========排行榜信息==========
                let rankings: RankingResult[] = [];
                if (item.ext?.detail_info?.bangdan_head?.ranking_show) {
                    rankings = item.ext.detail_info.bangdan_head.ranking_show.map((rank: BaiduMap.RankingInfo) => ({
                        name: rank.ranking,
                        rank: rank.rank_s,
                        score: rank.score,
                        short_name: rank.short_name,
                        type: rank.ranking_type
                    }));
                }

                // ==========活动事件==========
                let events: EventResult[] = [];
                if (item.ext?.detail_info?.event_notice) {
                    events.push({
                        title: item.ext.detail_info.event_notice.title,
                        content: item.ext.detail_info.event_notice.content,
                        img_url: item.ext.detail_info.event_notice.img_url,
                        start_end: item.ext.detail_info.event_notice.start_end
                    });
                }

                // ==========照片和街景==========
                const shop_hours_simple = item.shop_hours_simple;
                const photo_count = item.ext?.detail_info?.photo_num ? parseInt(item.ext.detail_info.photo_num, 10) : undefined;
                const has_indoor_map = item.ext?.detail_info?.indoor_map === '1';

                let has_street_view = false;
                let street_view_info: { pid: string; heading: number } | undefined = undefined;
                if (typeof item.pano === 'string' && item.pano) {
                    const firstPano = item.pano.split(';')[0];
                    const panoParts = firstPano.split(',');
                    if (panoParts.length >= 2 && panoParts[0]) {
                        has_street_view = true;
                        street_view_info = {
                            pid: panoParts[0],
                            heading: parseInt(panoParts[1] || '0', 10)
                        };
                    }
                }

                // ==========坐标信息==========
                let lng = 0, lat = 0;
                if (item.ext && item.ext.detail_info && item.ext.detail_info.point) {
                    lng = parseFloat(item.ext.detail_info.point.x);
                    lat = parseFloat(item.ext.detail_info.point.y);
                } else if (item.x && item.y) {
                    lng = parseFloat(item.x);
                    lat = parseFloat(item.y);
                } else if (item.point && item.point.x && item.point.y) {
                    lng = parseFloat(item.point.x);
                    lat = parseFloat(item.point.y);
                }

                // ==========地理数据==========
                const hasGeoData = !!(
                    (item.geo && item.geo.length > 0) ||
                    (item.ext && item.ext.detail_info && item.ext.detail_info.guoke_geo && item.ext.detail_info.guoke_geo.geo) ||
                    item.geo_type == 2
                );

                // ==========其他附加信息==========
                const additional_info: Record<string, unknown> = {};
                if (item.ext?.detail_info?.aoi_src_id) additional_info['aoi_src_id'] = item.ext.detail_info.aoi_src_id;
                if (item.ext?.detail_info?.navi_update_time) additional_info['navi_update_time'] = item.ext.detail_info.navi_update_time;
                if (item.ext?.detail_info?.official_url) additional_info['official_url'] = item.ext.detail_info.official_url;
                if (item.ext?.detail_info?.is_reservable) additional_info['is_reservable'] = item.ext.detail_info.is_reservable === '1';
                if (item.ext?.detail_info?.areaid) additional_info['area_id'] = item.ext.detail_info.areaid;
                if (item.ext?.detail_info?.entrance_price) additional_info['entrance_price'] = item.ext.detail_info.entrance_price;
                if (item.ext?.detail_info?.free) additional_info['is_free'] = item.ext.detail_info.free;

                // 构建详情URL
                const detailUrl = uid ? `https://map.baidu.com/?qt=ext&uid=${uid}` : "";

                return {
                    uid: uid,
                    name: name,
                    address: address,
                    area_name: area_name,
                    phone: phone,
                    tags: tags,
                    detail_type: detail_type,
                    rating: rating,
                    comment_count: comment_count,
                    price: price,
                    ticket_info: ticket_info,
                    opening_hours: opening_hours,
                    opening_hours_detail: opening_hours_detail,
                    shop_hours_simple: shop_hours_simple,
                    photo_count: photo_count,
                    has_street_view: has_street_view,
                    street_view_info: street_view_info,
                    has_indoor_map: has_indoor_map,
                    rankings: rankings,
                    events: events,
                    type: type,
                    has_geo_data: hasGeoData,
                    center: {
                        lng: lng,
                        lat: lat
                    },
                    detail_url: detailUrl,
                    additional_info: additional_info,
                    // 只在DEBUG模式下包含简化的原始数据
                    ...(currentLogLevel >= LOG_LEVELS.DEBUG ? {
                        raw_data: {
                            uid: item.uid,
                            name: item.name,
                            addr: item.addr,
                            x: item.x,
                            y: item.y,
                            geo_type: item.geo_type
                        }
                    } : {})
                } as AoiDataItem;
            });

            logger(LOG_LEVELS.DEBUG, `找到${potentialAois.length}个AOI结果`);

            return {
                success: true,
                keyword: keyword,
                city_name: params.city_name,
                total: potentialAois.length,
                aois: potentialAois
            };
        } catch (error) {
            logger(LOG_LEVELS.ERROR, `[search_aoi] 错误:`, error);
            logger(LOG_LEVELS.ERROR, `错误堆栈:`, error.stack);

            return {
                success: false,
                message: `搜索AOI失败: ${error.message}`,
                keyword: params.keyword,
                city_name: params.city_name,
                total: 0,
                aois: [] as AoiDataItem[]
            };
        }
    }

    /**
     * 获取AOI边界坐标
     */
    async function get_aoi_boundary(params: { uid: string }): Promise<AoiBoundaryResult> {
        try {
            const uid = params.uid;
            if (!uid) {
                throw new Error("AOI的UID不能为空");
            }

            // 构建URL - 使用百度地图地点详情接口
            const url = `https://map.baidu.com/?qt=ext&uid=${uid}`;

            logger(LOG_LEVELS.INFO, `获取AOI边界: ${uid}`);

            // 发起请求
            const result = await httpGet(url) as BaiduMap.ApiResponse;

            // 记录完整响应以便调试
            logger(LOG_LEVELS.DEBUG, `AOI边界结果结构:`, Object.keys(result || {}));

            // 检查是否有内容数据
            if (!result || !result.content || (Array.isArray(result.content) && result.content.length === 0)) {
                return {
                    success: false,
                    message: "未找到AOI边界数据",
                    uid: uid
                };
            }

            // content在新的API中可能是数组，也可能是对象
            const content = (Array.isArray(result.content) ? result.content[0] : result.content) as BaiduMap.ContentItem;

            if (!content) {
                return {
                    success: false,
                    message: "AOI内容数据为空",
                    uid: uid
                };
            }

            // 尝试多种可能的地理数据结构
            let geoData: string | ({ x?: string | number; y?: string | number; } | (string | number)[])[] | undefined = content.geo;
            if (!geoData && content.ext && content.ext.geo) {
                geoData = content.ext.geo;
            } else if (!geoData && content.geodata) {
                geoData = content.geodata as string | ({ x?: string | number; y?: string | number; } | (string | number)[])[];
            } else if (!geoData && content.guoke_geo && content.guoke_geo.geo) {
                geoData = content.guoke_geo.geo;
            }

            let boundary: Array<{ lng: number, lat: number }> = [];

            if (typeof geoData === 'string') {
                // 解析百度地图的geo字符串格式
                // 格式: "4|12957496.191084,4826145.210198;12958713.912140,4828039.128871|1-12957512.1751599,4827980.8872594,..."
                try {
                    const parts = geoData.split('|');
                    if (parts.length >= 3) {
                        // 第三部分包含详细的边界坐标
                        const boundaryPart = parts[2];
                        const coordinatePairs = boundaryPart.split(',');

                        // 每两个数字组成一个坐标点
                        for (let i = 0; i < coordinatePairs.length - 1; i += 2) {
                            const lng = parseFloat(coordinatePairs[i]);
                            const lat = parseFloat(coordinatePairs[i + 1]);

                            if (!isNaN(lng) && !isNaN(lat)) {
                                boundary.push({ lng, lat });
                            }
                        }

                        logger(LOG_LEVELS.DEBUG, `从geo字符串解析到${boundary.length}个边界点`);
                    }
                } catch (e) {
                    logger(LOG_LEVELS.ERROR, `解析geo字符串失败:`, e);
                }
            } else if (Array.isArray(geoData)) {
                // 尝试解析数组形式的边界
                boundary = geoData.map(point => ({
                    lng: parseFloat(String((Array.isArray(point) ? point[0] : point.x) || 0)),
                    lat: parseFloat(String((Array.isArray(point) ? point[1] : point.y) || 0))
                }));
                logger(LOG_LEVELS.DEBUG, `解析到${boundary.length}个边界点`);
            }

            return {
                success: true,
                uid: uid,
                name: content.name || "",
                address: content.addr || "",
                center: {
                    lng: parseFloat(content.x || (content.point && content.point.x) || '0'),
                    lat: parseFloat(content.y || (content.point && content.point.y) || '0')
                },
                boundary: boundary,
                point_count: boundary.length,
                // 只在DEBUG模式下包含简化的原始数据
                ...(currentLogLevel >= LOG_LEVELS.DEBUG ? {
                    raw_data: {
                        uid: content.uid,
                        name: content.name,
                        addr: content.addr,
                        x: content.x,
                        y: content.y,
                        geo_type: content.geo_type
                    }
                } : {})
            };
        } catch (error) {
            logger(LOG_LEVELS.ERROR, `[get_aoi_boundary] 错误:`, error);
            logger(LOG_LEVELS.ERROR, `错误堆栈:`, error.stack);

            return {
                success: false,
                message: `获取AOI边界失败: ${error.message}`,
                uid: params.uid
            };
        }
    }

    function wrap(coreFunction: (params: any) => Promise<any>) {
        return async (params: any) => {
            const result = await coreFunction(params);
            complete(result);
            return result;
        };
    }
    /**
     * 格式化AOI搜索结果为结构化文本
     */
    function formatAoiResultAsText(aoiResult: SearchAoiResult): string {
        if (!aoiResult.success) {
            return `AOI搜索失败: ${aoiResult.message || '未知错误'}`;
        }

        let output = `=== AOI搜索结果 ===\n`;
        output += `搜索关键词: ${aoiResult.keyword}\n`;
        output += `搜索城市: ${aoiResult.city_name || '全国'}\n`;
        output += `找到结果: ${aoiResult.total} 个\n\n`;

        if (aoiResult.aois && aoiResult.aois.length > 0) {
            aoiResult.aois.forEach((aoi, index) => {
                output += `--- 结果 ${index + 1} ---\n`;
                output += `名称: ${aoi.name}\n`;
                output += `类型: ${aoi.type || '未知'}\n`;
                output += `地址: ${aoi.address}\n`;

                if (aoi.area_name) {
                    output += `所属区域: ${aoi.area_name}\n`;
                }

                if (aoi.phone) {
                    output += `联系电话: ${aoi.phone}\n`;
                }

                if (aoi.tags) {
                    output += `标签: ${aoi.tags}\n`;
                }

                if (aoi.rating !== undefined) {
                    output += `评分: ${aoi.rating}/5.0`;
                    if (aoi.comment_count !== undefined) {
                        output += ` (${aoi.comment_count}条评论)`;
                    }
                    output += `\n`;
                }

                if (aoi.price) {
                    output += `价格信息: ${aoi.price}\n`;
                }

                if (aoi.ticket_info) {
                    output += `门票信息:\n`;
                    if (aoi.ticket_info.title) output += `  - 门票名称: ${aoi.ticket_info.title}\n`;
                    if (aoi.ticket_info.price) output += `  - 门票价格: ${aoi.ticket_info.price}\n`;
                    if (aoi.ticket_info.market_price) output += `  - 市场价: ${aoi.ticket_info.market_price}\n`;
                    if (aoi.ticket_info.sold_info) output += `  - 销售信息: ${aoi.ticket_info.sold_info}\n`;
                }

                if (aoi.opening_hours) {
                    output += `开放时间: ${aoi.opening_hours}\n`;
                } else if (aoi.shop_hours_simple) {
                    output += `当前状态: ${aoi.shop_hours_simple}\n`;
                }

                if (aoi.opening_hours_detail) {
                    const detail = aoi.opening_hours_detail;
                    if (detail.common_hours) {
                        output += `常规时间: ${detail.common_hours}\n`;
                    }
                    if (detail.festival_hours) {
                        output += `节假日时间: ${detail.festival_hours}\n`;
                    }
                }

                if (aoi.photo_count) {
                    output += `照片数量: ${aoi.photo_count} 张\n`;
                }

                const features: string[] = [];
                if (aoi.has_street_view) features.push('街景');
                if (aoi.has_indoor_map) features.push('室内地图');
                if (aoi.has_geo_data) features.push('边界数据');
                if (features.length > 0) {
                    output += `可用功能: ${features.join(', ')}\n`;
                }

                if (aoi.rankings && aoi.rankings.length > 0) {
                    output += `排行榜信息:\n`;
                    aoi.rankings.slice(0, 3).forEach(rank => {
                        if (rank.name && rank.rank) {
                            output += `  - ${rank.name}: ${rank.rank}`;
                            if (rank.score) output += ` (评分: ${rank.score})`;
                            output += `\n`;
                        }
                    });
                }

                if (aoi.events && aoi.events.length > 0 && aoi.events[0].title) {
                    output += `当前活动:\n`;
                    aoi.events.forEach(event => {
                        if (event.title) {
                            output += `  - ${event.title}`;
                            if (event.start_end) output += ` (${event.start_end})`;
                            output += `\n`;
                            if (event.content) output += `    ${event.content}\n`;
                        }
                    });
                }

                output += `坐标: 经度 ${aoi.center.lng}, 纬度 ${aoi.center.lat}\n`;
                output += `详情链接: ${aoi.detail_url}\n`;

                if (Object.keys(aoi.additional_info).length > 0) {
                    output += `其他信息: `;
                    const info: string[] = [];
                    for (const [key, value] of Object.entries(aoi.additional_info)) {
                        if (key === 'is_free' && value === 2) {
                            info.push('收费景点');
                        } else if (key === 'is_free' && value === 1) {
                            info.push('免费景点');
                        } else if (key === 'area_id') {
                            info.push(`区域ID: ${value}`);
                        } else {
                            info.push(`${key}: ${value}`);
                        }
                    }
                    output += info.join(', ') + `\n`;
                }

                output += `\n`;
            });

            if (aoiResult.boundary) {
                output += `=== 边界信息 ===\n`;
                output += `边界点数: ${aoiResult.boundary.point_count || 0}\n`;
                if (aoiResult.boundary.center) {
                    output += `中心坐标: 经度 ${aoiResult.boundary.center.lng}, 纬度 ${aoiResult.boundary.center.lat}\n`;
                }
                output += `\n`;
            }
        }

        return output;
    }

    /**
     * 格式化路线规划结果为结构化文本
     */
    function formatRouteResultAsText(routeResult: RouteResult): string {
        if (!routeResult.success) {
            return `路线规划失败: ${routeResult.message || '未知错误'}`;
        }

        let output = `=== 路线规划结果 ===\n`;

        if (routeResult.current_location) {
            output += `当前位置: 经度 ${routeResult.current_location.lng}, 纬度 ${routeResult.current_location.lat}\n`;
            if (routeResult.current_location.address) {
                output += `当前地址: ${routeResult.current_location.address}\n`;
            }
        }

        if (routeResult.destination) {
            output += `目的地: ${routeResult.destination.name}\n`;
            output += `目的地址: ${routeResult.destination.address}\n`;
            output += `目的坐标: 经度 ${routeResult.destination.location.lng}, 纬度 ${routeResult.destination.location.lat}\n`;
        }

        output += `\n`;

        if (routeResult.all_routes) {
            output += `=== 所有交通方式 ===\n`;

            // 驾车路线
            if (routeResult.all_routes.driving) {
                const driving = routeResult.all_routes.driving;
                output += `🚗 驾车路线:\n`;
                output += `  距离: ${driving.estimated_distance}\n`;
                output += `  时间: ${driving.estimated_duration}\n`;
                output += `  建议: ${driving.suggestion}\n\n`;
            }

            // 步行路线
            if (routeResult.all_routes.walking) {
                const walking = routeResult.all_routes.walking;
                output += `🚶 步行路线:\n`;
                output += `  距离: ${walking.estimated_distance}\n`;
                output += `  时间: ${walking.estimated_duration}\n`;
                output += `  建议: ${walking.suggestion}\n\n`;
            }

            // 公交路线
            if (routeResult.all_routes.transit) {
                const transit = routeResult.all_routes.transit;
                output += `🚌 公共交通:\n`;
                output += `  距离: ${transit.estimated_distance}\n`;
                output += `  时间: ${transit.estimated_duration}\n`;
                output += `  建议: ${transit.suggestion}\n\n`;
            }
        }

        return output;
    }


    /**
     * 测试格式化函数
     */
    async function main(): Promise<string> {
        let output = "";
        output += "========== 百度地图格式化函数测试 ==========\n\n";

        try {
            // 测试AOI搜索格式化
            output += "[1] 测试AOI搜索格式化...\n";
            const aoiResult = await search_aoi({
                keyword: "长安大学",
                city_name: "西安"
            });
            output += formatAoiResultAsText(aoiResult) + "\n";

            await Tools.System.sleep(1000);

            // 测试路径规划格式化
            output += "[3] 测试路径规划格式化...\n";
            const routeResult = await planRoute({
                destination: "长安大学",
                city_name: "西安"
            });
            output += formatRouteResultAsText(routeResult) + "\n";

            output += "========== 格式化测试完成 ==========\n";

            logger(LOG_LEVELS.INFO, output);
            return output;
        } catch (error) {
            return `测试过程中发生错误: ${error.message}`;
        }
    }

    /**
     * 获取用户当前位置
     * 使用系统提供的位置API获取当前位置坐标
     */
    async function getCurrentLocation(): Promise<{ lng: number, lat: number } | undefined> {
        try {
            logger(LOG_LEVELS.INFO, "正在获取用户当前位置...");
            const locationResult = await Tools.System.getLocation();

            if (!locationResult) {
                logger(LOG_LEVELS.ERROR, "获取位置失败:", "未知错误");
                return undefined;
            }

            // 获取成功，返回经纬度
            return {
                lng: locationResult.longitude,
                lat: locationResult.latitude
            };
        } catch (error) {
            logger(LOG_LEVELS.ERROR, "获取位置出错:", error.message);
            return undefined;
        }
    }

    /**
     * 将百度墨卡托坐标转换为WGS-84坐标系下的经纬度（近似）
     * @param mercatorLng 墨卡托x坐标
     * @param mercatorLat 墨卡托y坐标
     * @returns {{lng: number, lat: number}} WGS-84经纬度
     */
    function convertMercatorToLatLng(mercatorLng: number, mercatorLat: number): { lng: number, lat: number } {
        const lng = (mercatorLng / 20037508.34) * 180;
        let lat = (mercatorLat / 20037508.34) * 180;
        lat = (180 / Math.PI) * (2 * Math.atan(Math.exp(lat * Math.PI / 180)) - Math.PI / 2);
        // 此转换未处理BD-09到WGS-84的偏移，但对于消除大的距离计算错误已经足够
        return { lng: lng, lat: lat };
    }

    /**
     * 智能路线规划
     * 高级封装函数，根据用户当前位置和目的地名称，提供导航信息
     */
    async function planRoute(params: {
        destination: string;    // 目的地名称
        city_name?: string;     // 城市名称，辅助目的地查找
    }): Promise<RouteResult> {
        try {
            // 步骤 1: 获取当前位置和搜索目的地（所有模式通用）
            const currentLocation = await getCurrentLocation();
            if (!currentLocation) {
                return { success: false, message: "无法获取当前位置信息" };
            }

            const searchResults = await search_aoi({
                keyword: params.destination,
                city_name: params.city_name
            });

            if (!searchResults.success || !searchResults.aois || searchResults.aois.length === 0) {
                return {
                    success: false,
                    message: `未能找到目的地: ${params.destination}`,
                    current_location: currentLocation
                };
            }

            const destination = searchResults.aois[0];
            const destLocation = destination.center;

            if (!destLocation || !destLocation.lng || !destLocation.lat) {
                return {
                    success: false,
                    message: `目的地坐标信息无效: ${params.destination}`,
                    current_location: currentLocation
                };
            }

            const destLatLng = convertMercatorToLatLng(destLocation.lng, destLocation.lat);
            logger(LOG_LEVELS.DEBUG, `目的地 "${destination.name}" 墨卡托坐标:`, destLocation);
            logger(LOG_LEVELS.DEBUG, `转换后的经纬度:`, destLatLng);

            const distance = calculateDistance(
                currentLocation.lat,
                currentLocation.lng,
                destLatLng.lat,
                destLatLng.lng
            );

            // 帮助函数，用于计算特定交通方式的路线详情
            const getRouteDetailsForMode = async (mode: "driving" | "walking" | "transit"): Promise<RouteDetails> => {
                let cityCode = "1";
                if (params.city_name) {
                    cityCode = await getCityCode(params.city_name);
                }
                const navUrl = `https://api.map.baidu.com/direction?origin=${currentLocation.lat},${currentLocation.lng}&destination=${destLatLng.lat},${destLatLng.lng}&mode=${mode}&region=${cityCode}&output=html`;

                return {
                    estimated_distance: `${(distance / 1000).toFixed(2)}公里`,
                    estimated_duration: estimateDuration(distance, mode),
                    transport_mode: mode,
                    navigation_url: navUrl,
                    suggestion: getSuggestion(distance, mode)
                };
            };

            // 并发获取所有交通方式的路线
            logger(LOG_LEVELS.INFO, "获取所有模式的路线规划...");
            const modes: ("driving" | "walking" | "transit")[] = ["driving", "walking", "transit"];
            const allRoutesPromises = modes.map(mode => getRouteDetailsForMode(mode));
            const routesResults = await Promise.all(allRoutesPromises);

            const all_routes = {
                driving: routesResults[0],
                walking: routesResults[1],
                transit: routesResults[2]
            };

            return {
                success: true,
                current_location: currentLocation,
                destination: {
                    name: destination.name,
                    address: destination.address,
                    location: destLatLng
                },
                all_routes: all_routes
            };
        } catch (error) {
            logger(LOG_LEVELS.ERROR, `[planRoute] 错误:`, error);
            return {
                success: false,
                message: `路线规划失败: ${error.message}`
            };
        }
    }

    // 工具函数：计算两点间距离（米）
    function calculateDistance(lat1: number, lng1: number, lat2: number, lng2: number): number {
        const R = 6371000; // 地球半径，单位米
        const dLat = (lat2 - lat1) * Math.PI / 180;
        const dLng = (lng2 - lng1) * Math.PI / 180;
        const a =
            Math.sin(dLat / 2) * Math.sin(dLat / 2) +
            Math.cos(lat1 * Math.PI / 180) * Math.cos(lat2 * Math.PI / 180) *
            Math.sin(dLng / 2) * Math.sin(dLng / 2);
        const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
        return R * c;
    }

    // 工具函数：格式化时间为可读字符串
    function formatDuration(minutes: number): string {
        if (minutes < 60) {
            return `约${Math.ceil(minutes)}分钟`;
        }
        const hours = Math.floor(minutes / 60);
        const remainingMinutes = Math.ceil(minutes % 60);
        if (remainingMinutes === 0) {
            return `约${hours}小时`;
        }
        return `约${hours}小时${remainingMinutes}分钟`;
    }

    // 工具函数：估算行程时间
    function estimateDuration(distance: number, mode: string): string {
        let minutes = 0;
        const distanceInKm = distance / 1000;

        switch (mode) {
            case "walking":
                // 步行速度约 4 km/h (15 min/km)
                minutes = distanceInKm * 15;
                break;
            case "transit":
                // 公共交通，分段估算
                if (distanceInKm < 10) { // 市内
                    minutes = distanceInKm * 4; // 15 km/h
                } else if (distanceInKm < 100) {
                    minutes = distanceInKm * 2; // 30 km/h
                } else { // 长途
                    minutes = distanceInKm * 1; // 60 km/h
                }
                break;
            case "driving":
            default:
                // 驾车，分段估算
                if (distanceInKm < 10) { // 市区
                    minutes = distanceInKm * 2; // 30 km/h
                } else if (distanceInKm < 100) {
                    minutes = distanceInKm * 1.2; // 50 km/h
                } else { // 高速
                    minutes = distanceInKm * 0.75; // 80 km/h
                }
                break;
        }
        return formatDuration(minutes);
    }

    // 工具函数：根据距离和交通方式给出建议
    function getSuggestion(distance: number, mode: string): string {
        if (distance < 500) {
            return "目的地非常近，步行即可到达";
        } else if (distance < 2000) {
            return "距离适中，可步行或乘坐短途交通工具";
        } else if (distance < 5000) {
            return "距离较远，建议使用公共交通工具";
        } else {
            return "目的地较远，建议驾车或使用公共交通工具";
        }
    }

    /**
     * 创建HTTP客户端
     * 使用OkHttp库实现网络请求
     */
    function createHttpClient() {
        return OkHttp.newBuilder()
            .connectTimeout(10000)
            .readTimeout(30000)
            .writeTimeout(15000)
            .followRedirects(true)
            .build();
    }

    /**
     * 获取城市编码
     * @param cityName 城市名称，如"杭州"、"南京"等
     * @returns 返回城市编码字符串，如果找不到返回默认编码"1"(全国)
     */
    async function getCityCode(cityName: string): Promise<string> {
        try {
            logger(LOG_LEVELS.INFO, `开始查询城市编码: ${cityName}`);

            // 如果已在本地映射表中，直接返回
            if (CITY_CODES[cityName]) {
                logger(LOG_LEVELS.DEBUG, `本地映射表中找到城市"${cityName}"的编码:`, CITY_CODES[cityName]);
                return CITY_CODES[cityName];
            }

            // 使用百度地图API查询城市信息
            const encodedCityName = encodeURIComponentSafe(cityName);
            const url = `https://map.baidu.com/?newmap=1&qt=s&wd=${encodedCityName}&c=1`;

            logger(LOG_LEVELS.INFO, `发送请求获取城市编码:`, url);
            const result = await httpGet(url) as BaiduMap.ApiResponse;

            // 尝试从不同路径提取城市编码
            let cityCode = "1"; // 默认值

            // 路径1: current_city.code
            if (result && result.current_city && result.current_city.code) {
                cityCode = result.current_city.code.toString();
                logger(LOG_LEVELS.DEBUG, `从current_city中找到城市编码:`, cityCode);
            }
            // 路径2: content[].area_code 或 city_id
            else if (result && result.content && Array.isArray(result.content) && result.content.length > 0) {
                for (const item of result.content) {
                    if (item.area_code) {
                        cityCode = item.area_code.toString();
                        logger(LOG_LEVELS.DEBUG, `从content[].area_code中找到城市编码:`, cityCode);
                        break;
                    } else if (item.city_id) {
                        cityCode = item.city_id.toString();
                        logger(LOG_LEVELS.DEBUG, `从content[].city_id中找到城市编码:`, cityCode);
                        break;
                    }
                }
            }
            // 路径3: result_code
            else if (result && result.result && result.result.code) {
                cityCode = result.result.code.toString();
                logger(LOG_LEVELS.DEBUG, `从result.code中找到城市编码:`, cityCode);
            }
            // 路径4: result.city_id
            else if (result && result.result && result.result.city_id) {
                cityCode = result.result.city_id.toString();
                logger(LOG_LEVELS.DEBUG, `从result.city_id中找到城市编码:`, cityCode);
            }

            // 如果没找到，尝试第二个API端点
            if (cityCode === "1") {
                // 使用城市搜索API
                const secondUrl = `https://map.baidu.com/?qt=cur&wd=${encodedCityName}`;
                logger(LOG_LEVELS.DEBUG, `未找到编码，尝试第二个API端点:`, secondUrl);

                try {
                    // 增加延迟避免请求过快
                    await Tools.System.sleep(500);

                    const secondResult = await httpGet(secondUrl);
                    if (secondResult && secondResult.current_city && secondResult.current_city.code) {
                        cityCode = secondResult.current_city.code.toString();
                        logger(LOG_LEVELS.DEBUG, `从第二个API获取到城市编码:`, cityCode);
                    }
                } catch (e) {
                    logger(LOG_LEVELS.ERROR, `第二个API请求失败:`, e);
                }
            }

            // 如果仍未找到，使用默认值
            if (cityCode === "1") {
                logger(LOG_LEVELS.INFO, `未能找到城市"${cityName}"的编码，使用默认编码"1"(全国)`);
            } else {
                // 找到编码后，可以临时添加到CITY_CODES中供本次会话使用
                CITY_CODES[cityName] = cityCode;
                logger(LOG_LEVELS.DEBUG, `已将城市"${cityName}"的编码${cityCode}添加到临时映射表`);
            }

            return cityCode;
        } catch (error) {
            logger(LOG_LEVELS.ERROR, `获取城市编码失败:`, error);
            return "1"; // 出错时返回默认编码(全国)
        }
    }

    return {

        // 格式化工具函数
        formatAoiResultAsText: wrap(async (param: any) => formatAoiResultAsText(await search_aoi(param))),
        formatRouteResultAsText: wrap(async (param: any) => formatRouteResultAsText(await planRoute(param))),

        // 测试函数
        main: wrap(main),
    };
})();

// 格式化工具函数导出
exports.search_aoi = baiduMap.formatAoiResultAsText;
exports.planRoute = baiduMap.formatRouteResultAsText;

// 测试函数导出
exports.main = baiduMap.main;