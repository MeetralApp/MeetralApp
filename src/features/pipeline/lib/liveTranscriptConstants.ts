/** Interim transcript events kept per direction (streaming only). */
export const TRANSCRIPT_INTERIM_BUFFER = 64;

/** Segments hydrated on live mount / reload per direction. */
export const LIVE_SEGMENT_INITIAL_WINDOW = 150;

/** Segments fetched per load-older request. */
export const LIVE_SEGMENT_PAGE_SIZE = 100;

/** Fallback scroll-top px to trigger load-more. */
export const LIVE_LOAD_MORE_TOP_THRESHOLD_PX = 120;

/** Virtual index ≤ this value triggers load-more (with overscan). */
export const LOAD_MORE_INDEX_THRESHOLD = 5;
