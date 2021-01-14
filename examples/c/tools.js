import { Rule } from "gear";
import { exec, sleep } from "system";

export default function({obj_dir, lib_dir, inc_dir, bin_dir}) {
    function cc(src) {
        let obj = obj_dir.output(src.name + '.o');
        Rule(src, obj, async function compile() {
            let src = this.inputs[0].name;
            let obj = this.outputs[0].name;
            console.log(`cc ${obj} < ${src}`);
            await sleep(500); // artifical delay
            let {status, output, error} = await exec({
                cmd: "gcc",
                args: ["-I", inc_dir.path, "-c", "-o", obj, src],
            });
            if (output) {
                console.warn(`cc output: ${output}`);
            }
            if (error) {
                console.error(`cc error: ${error}`);
            }
            if (status && status != 0) {
                throw new Error(`Error when compiling ${src} Status: ${status}`);
            }
        });
        return obj.input;
    }

    // Archiver
    function ar(name, objs) {
        let lib = lib_dir.output(`lib${name}.a`);
        Rule(objs, lib, async function link() {
            let objs = this.inputs.map(obj => obj.name);
            let lib = this.outputs[0].name;
            console.log(`ar ${lib} < ${objs.join(" ")}`);
            await sleep(700); // artifical delay
            let {status, output, error} = await exec({
                cmd: "gcc-ar",
                args: ["cr", lib, ...objs],
            });
            if (output) {
                console.warn(`ar output: ${output}`);
            }
            if (error) {
                console.error(`ar error: ${error}`);
            }
            if (status && status != 0) {
                throw new Error(`Error when archiving ${name} Status: ${status}`);
            }
        });
        return lib.input;
    }

    // Linker
    function ld(name, objs, libs = []) {
        let bin = bin_dir.output(name);
        let libs_flags = libs.map(lib => `-l${lib}`);
        Rule(objs, bin, async function link() {
            let objs = this.inputs.map(obj => obj.name);
            let bin = this.outputs[0].name;
            console.log(`ld ${bin} < ${objs.join(" ")}`);
            await sleep(1000); // artifical delay
            let {status, output, error} = await exec({
                cmd: "gcc",
                args: ["-o", bin, ...objs, ...libs_flags],
            });
            if (output) {
                console.warn(`cc output: ${output}`);
            }
            if (error) {
                console.error(`cc error: ${error}`);
            }
            if (status && status != 0) {
                throw new Error(`Error when linking ${name} Status: ${status}`);
            }
        });
        return bin.input;
    }

    return {cc, ar, ld};
}
