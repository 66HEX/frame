declare module 'bun:test' {
	export function describe(name: string, fn: () => void): void;
	export function test(name: string, fn: () => void | Promise<void>): void;
	export const expect: {
		<T>(actual: T): {
			toBe(expected: T): void;
			toEqual(expected: unknown): void;
			toBeCloseTo(expected: number, precision?: number): void;
		};
	};
}
