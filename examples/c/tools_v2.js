import { Compiler } from "toolchain";

export default async function({obj_dir, lib_dir, bin_dir, inc_dirs = [], cdefs = {}, cflags = []}) {
    const compiler = (await Compiler.detect({ compiler: "clang" })).config({
        flags: cflags,
        cc: {
            defs: cdefs,
            dirs: inc_dirs.map(dir => dir.path),
        },
    });

    // C Compiler
    async function cc(src) {
        return await compiler.compile(obj_dir, src);
    }

    // Archiver
    async function ar(name, objs) {
        return (await compiler.link(lib_dir, name, objs, { type: "static" })).out;
    }

    // Linker
    async function ld(name, objs, libs = []) {
        return (await compiler.config({
            link: {
                libs,
            }
        }).link(bin_dir, name, objs)).out;
    }

    return {cc, ar, ld};
}
