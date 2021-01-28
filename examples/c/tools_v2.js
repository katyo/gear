import { Compiler } from "toolchain";

export default async function({obj_dir, lib_dir, bin_dir, inc_dirs = [], cdefs = {}, cflags = []}) {
    const compiler = (await Compiler.detect({ compiler: "clang" })).config({
        common: {
            flags: cflags,
        },
        compile: {
            defs: cdefs,
            dirs: inc_dirs.map(dir => dir.path),
        },
    });

    // C Compiler
    async function cc(src) {
        return await compiler.cc(obj_dir, src);
    }

    // Archiver
    async function ar(name, objs) {
        return await compiler.ar(lib_dir, name, objs);
    }

    // Linker
    async function ld(name, objs, libs = []) {
        console.log(objs);
        return await compiler.config({
            link: {
                libs,
            }
        }).ld(bin_dir, name, objs);
    }

    return {cc, ar, ld};
}
