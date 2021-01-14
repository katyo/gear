import {join as pathJoin} from 'path';
import Server from 'fastify';
import Static from 'fastify-static';
import {FastifySSEPlugin} from 'fastify-sse-v2';
import EventIterator from 'event-iterator';
import {EventEmitter} from 'events';

import { data as rules, events, Event, EventTag } from './data';

const bus = new EventEmitter();

const app = Server();

app.register(Static, {
    root: pathJoin(process.cwd(), 'public')
});
app.register(FastifySSEPlugin);

app.get('/rules', async (req, res) => {
    res.type('application/json').code(200);
    return rules;
});
app.get('/events', (req, res) => {
    res.sse(new EventIterator(
        ({push}) => {
            bus.addListener("sse", push);
            return () => bus.removeListener("sse", push);
        }
    ));
});

app.listen(8888, (err, address) => {
    if (err) throw err;
    console.log(`Server listening on ${address}`);
});

function play(events: Event[], index: number = 0) {
    if (index >= events.length) {
        index = 0;
    }

    const { delay, event, data } = events[index++];

    setTimeout(() => {
        if (event == EventTag.RuleStateChange) {
            rules.rules[data.rule].state = data.state;
        }
        bus.emit('sse', {data: data ? JSON.stringify(data) : undefined, event });

        play(events, index);
    }, delay);
}

play(events);
