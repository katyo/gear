declare module "gear" {
    class Input {
        readonly name: string;
    }

    class Output extends Input {
        readonly input: Input;
    }

    class AnyRule {
        inputs: Input[];
        readonly outputs: Output[];
    }

    function Rule(inputs: Input[], outputs: Output[], func?: (this: AnyRule) => Promise<void>): AnyRule;
    function Rule(outputs: Output[], inputs: Input[], func?: (this: AnyRule) => Promise<void>): AnyRule;
    function Rule(func: (this: AnyRule) => Promise<void>, outputs: Output[], inputs: Input[]): AnyRule;

    class Goal {
        inputs: Input[];
        readonly input: Input;
    }

    class Directory {
        readonly path: string;
        readonly parent?: Directory;
        readonly input: Input;
        readonly output: Output;
        child(path: string): Directory;
    }

    class Scope {
        readonly name: string;
        description: string;
        scope(name: string, description?: string): Scope;
        input(name: string): Input;
        output(name: string): Output;
        goal(name: string, description?: string, cb?: (this: Goal) => Promise<void>);
        goal(name: string, cb: (this: Goal) => Promise<void>, description?: string);
    }
}
