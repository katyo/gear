import { Directory, Scope } from "gear";

declare interface Console {
    log(...args: any[]): void;
    info(...args: any[]): void;
    warn(...args: any[]): void;
    error(...args: any[]): void;
    debug(...args: any[]): void;
    trace(...args: any[]): void;
}

declare var console: Console;
declare var base: Directory;
declare var dest: Directory;
declare var scope: Scope;
