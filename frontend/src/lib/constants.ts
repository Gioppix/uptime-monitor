import type { components } from './api/schema';

export type Region = components['schemas']['Region'];
export type Method = components['schemas']['Method'];

export const REGION_LABELS: { [K in Region]: string } = {
    Fsn1: 'Falkenstein',
    Hel1: 'Helsinki',
    Nbg1: 'Nuremberg'
};

const METHODS: { [K in Method]: null } = {
    GET: null,
    DELETE: null,
    HEAD: null,
    POST: null,
    PUT: null
};

export const ALL_METHODS: Method[] = Object.entries(METHODS).map(([m, _]) => m);

export const ALL_REGIONS: Region[] = Object.entries(REGION_LABELS).map(([m, _]) => m);
