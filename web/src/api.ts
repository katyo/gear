import { readable, derived } from 'svelte/store';

export const enum RuleState {
    Processed = 'processed',
    Scheduled = 'scheduled',
    Processing = 'processing',
}

export interface RuleItem {
		state: RuleState,
		inputs: string[],
		outputs: string[],
}

export interface RulesMap {
    [id: string]: RuleItem,
}

export interface ArtifactItem {
    goal: boolean,
    rule?: string,
}

export interface ArtifactsMap {
    [name: string]: ArtifactItem;
}

export type GoalsList = string[];

function prepare_map(rules: RulesMap, goals: GoalsList): ArtifactsMap {
    let artifacts: ArtifactsMap = {};

    for (let rule_id in rules) {
        let rule = rules[rule_id];
        for (let artifact_name of rule.inputs) {
            if (!(artifact_name in artifacts)) {
                artifacts[artifact_name] = {
                    goal: goals.indexOf(artifact_name) > -1,
                };
            }
        }
        for (let artifact_name of rule.outputs) {
            if (!(artifact_name in artifacts)) {
                artifacts[artifact_name] = {
                    goal: goals.indexOf(artifact_name) > -1,
                };
            }
            artifacts[artifact_name].rule = rule_id;
        }
    }

    return artifacts;
}

export const enum StoreError {
    Disconnected = 1,
    InvalidData = 2,
}

export const enum EventTag {
    RulesUpdate = 'rules-update',
    RuleStateChange = 'rule-state',
}

export interface RulesData {
    rules: RulesMap,
    goals: GoalsList,
}

export interface StateData {
    rule: string,
    state: RuleState,
}

interface ErrorData {
    error: StoreError,
}

const $connection = readable<RulesData | StateData | ErrorData | {}>({}, (set) => {
    function fetchRules() {
        fetch(`/rules`).then(resp => resp.json(), error => {
            set({error: StoreError.Disconnected});
        }).then(set, error => {
            set({error: StoreError.InvalidData});
        });
    }

    const source = new EventSource(
        `/events`,
    );

    source.onopen = () => {
        fetchRules();
    };

    source.onerror = () => {
        set({error: StoreError.Disconnected});
    };

    source.addEventListener(EventTag.RulesUpdate, () => {
        fetchRules();
    });

    source.addEventListener(EventTag.RuleStateChange, (event: MessageEvent) => {
        let data;
        try {
            data = JSON.parse(event.data);
            if (typeof data.rule != 'string' || typeof data.state != 'string') {
                throw 0;
            }
        } catch (error) {
            data.error = StoreError.InvalidData;
            set(data);
            return;
        }
        set(data);
    });

    () => {
        source.close();
    };
});

export const error = (() => {
    let _error;
    return derived($connection, ({error}, set) => {
        if (error != _error) {
            _error = error;
            set(error);
        }
    });
})();

export const rules = (() => {
    let _rules;
    return derived($connection, ({rules, rule, state}, set) => {
        if (rules) {
            _rules = rules;
        } else if (rule && state) {
            _rules[rule].state = state;
        } else {
            return;
        }
        set(_rules);
    });
})();

export const artifacts = (() => {
    let _rules;
    return derived($connection, ({rules, goals}, set) => {
        if (rules && rules !== _rules) {
            _rules = rules;
            set(prepare_map(rules, goals));
        }
    });
})();
