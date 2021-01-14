import {RulesData, RuleState, EventTag} from '../src/api';
export {EventTag};

export const data: RulesData = {
    "goals": ["run.bin","build.lib","clean","build.bin","build"],
    "rules": {
        "5983926933566305584": {
            "state": RuleState.Processed,
            "inputs": ["target/debug/hello"],
            "outputs": ["run.bin"]
        },
        "910278429342045793": {
            "state": RuleState.Processed,
            "inputs": ["target/debug/libhello.a"],
            "outputs": ["build.lib"]
        },
        "16660411825264012050": {
            "state": RuleState.Processed,
            "inputs": [],
            "outputs": ["clean"]
        },
        "3118746951582511172": {
            "state": RuleState.Processed,
            "inputs": ["target/debug/hello"],
            "outputs": ["build.bin"]
        },
        "4581478379212543693": {
            "state": RuleState.Processed,
            "inputs": ["build.lib", "build.bin"],
            "outputs": ["build"]
        },
        "9590944225790538900": {
            "state": RuleState.Processed,
            "inputs": ["src/main.c"],
            "outputs": ["target/debug/obj/src/main.c.o"]
        },
        "12858439256136511092": {
            "state": RuleState.Processed,
            "inputs": ["target/debug/obj/src/main.c.o", "target/debug/libhello.a"],
            "outputs": ["target/debug/hello"]
        },
        "9956545090116390919": {
            "state": RuleState.Processed,
            "inputs": ["src/hello.c"],
            "outputs": ["target/debug/obj/src/hello.c.o"]
        },
        "12718070188570325500": {
            "state": RuleState.Processed,
            "inputs": ["src/bye.c"],
            "outputs": ["target/debug/obj/src/bye.c.o"]
        },
        "8978663227669744191": {
            "state": RuleState.Processed,
            "inputs": ["target/debug/obj/src/hello.c.o", "target/debug/obj/src/bye.c.o"],
            "outputs":["target/debug/libhello.a"]
        }
    }
};

export type Event = {
    delay: number,
} & ({
    event: EventTag.RulesUpdate
    data?: undefined,
} | {
    event: EventTag.RuleStateChange,
    data: {
        rule: string,
        state: RuleState
    }
});

export const events: Event[] = [
    { delay: 1500, event: EventTag.RulesUpdate },
    // main.o
    { delay: 300, event: EventTag.RuleStateChange, data: { rule: "9590944225790538900", state: RuleState.Processing } },
    // hello
    { delay: 0, event: EventTag.RuleStateChange, data: { rule: "12858439256136511092", state: RuleState.Scheduled } },
    // build.bin
    { delay: 0, event: EventTag.RuleStateChange, data: { rule: "3118746951582511172", state: RuleState.Scheduled } },
    // build
    { delay: 0, event: EventTag.RuleStateChange, data: { rule: "4581478379212543693", state: RuleState.Scheduled } },
    // main.o
    { delay: 2000, event: EventTag.RuleStateChange, data: { rule: "9590944225790538900", state: RuleState.Processed } },
    // hello
    { delay: 0, event: EventTag.RuleStateChange, data: { rule: "12858439256136511092", state: RuleState.Processing } },
    // hello
    { delay: 3000, event: EventTag.RuleStateChange, data: { rule: "12858439256136511092", state: RuleState.Processed } },
    // build.bin
    { delay: 0, event: EventTag.RuleStateChange, data: { rule: "3118746951582511172", state: RuleState.Processed } },
    // build
    { delay: 0, event: EventTag.RuleStateChange, data: { rule: "4581478379212543693", state: RuleState.Processed } },
];
