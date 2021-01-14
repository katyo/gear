import { writable, Writable } from 'svelte/store';

interface Storage {
    getItem(key: string): string | undefined;
    setItem(key: string, value: string);
}

function Storage<T>(storage: Storage): (key: string, initial?: T) => Writable<T> {
    return (key: string, initial?: T) => {
        let data = storage.getItem(key);
        if (typeof data == 'string') {
            try {
                initial = JSON.parse(data);
            } catch (e) {}
        }
        const store = writable(initial);
        store.subscribe(data => {
            storage.setItem(key, JSON.stringify(data));
        });
        return store;
    };
}

export const SessionStore = Storage(sessionStorage);
export const LocalStore = Storage(localStorage);
