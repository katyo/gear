declare module "system" {
    /// Sleep for milliseconds
    export function sleep(msec: number): Promise<void>;

    /// Check file existing
    export function is_file(path: string): Promise<boolean>;

    /// Check directory existing
    export function is_dir(path: string): Promise<boolean>;

    /// Check path existing
    export function exists(path: string): Promise<boolean>;

    /// Remove file or directory
    export function remove(path: string): Promise<boolean>;

    export interface ExecOptions {
        /// Command name (ex. gcc/clang/cat etc.)
        cmd: string;
        /// Additional command-line options
        args?: string[];
        /// Additional environment variables
        envs?: { [name: string]: string; };
        /// Working directory
        cwd?: string;
        /// Data to feed via stdin
        input?: string;
    }

    export interface ExecResult {
        /// Status code (undefined in case of abnormal termination)
        status?: number;
        /// Data received from stdout
        output: string;
        /// Data received from stderr
        error: string;
    }

    /// Execute arbitrary program
    export function exec(opts: ExecOptions): Promise<ExecResult>;
}
