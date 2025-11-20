// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
    namespace App {
        // interface Error {}
        // interface Locals {}
        // interface PageData {}
        // interface PageState {}
        // interface Platform {}
    }
}

// https://github.com/sindresorhus/type-fest/issues/649
type ObjectEntry<BaseType> = [keyof BaseType, BaseType[keyof BaseType]];
type ObjectEntries<BaseType> = Array<ObjectEntry<BaseType>>;

declare global {
    interface ObjectConstructor {
        entries<T extends object>(o: T): ObjectEntries<T>;
    }
}

export {};
