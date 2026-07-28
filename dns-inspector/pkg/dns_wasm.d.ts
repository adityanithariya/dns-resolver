/* tslint:disable */
/* eslint-disable */

export function decode_message(bytes: Uint8Array): any;

export function encode_query(id: number, name: string, qtype: number): Uint8Array;

export function format_message(bytes: Uint8Array): string;

export function version(): string;
