export interface FetchTemplateMapEntry {
  name: string
  url: string
}

/** Fetch remote template sources; pass the result to `Environment#setTemplateMap`. */
export declare function fetchTemplateMap(
  entries: FetchTemplateMapEntry[],
): Promise<Record<string, string>>
