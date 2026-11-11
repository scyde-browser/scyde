/* tslint:disable */
/* eslint-disable */

export class ShieldEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Cosmetic resources for a page URL as JSON (hide selectors, injected scriptlets).
     */
    cosmetic(url: string): string;
    /**
     * Generic class/id selectors to hide, given classes and ids seen in the DOM.
     */
    hidden_selectors(classes: string[], ids: string[], exceptions_json: string): string[];
    /**
     * Builds an engine from newline-separated filter list text.
     */
    constructor(rules: string);
    /**
     * Serialized engine size, to estimate the `engine.dat` cost.
     */
    serialized_len(): number;
    /**
     * Returns true if the request must be blocked.
     */
    should_block(url: string, source_url: string, request_type: string): boolean;
}
