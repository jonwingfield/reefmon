import * as React from "react";
import { compare, Comparable } from "../utils";

type TemperatureUnit = 'C' | 'F';

class TemperatureValue<Unit extends TemperatureUnit> implements Temperature<TemperatureUnit> {
    value: number;    
    unit: Unit;

    constructor(value: number, unit: Unit) {
        if (isNaN(value)) {
            throw new Error("Not a number: " + value)
        }
        this.value = value;
        this.unit = unit;
    }

    toString() {
        return this.value + this.unit;
    }

    compareTo(other: Temperature<Unit>) {
        return compare(this.value, other.value);
    }
}

export interface Temperature<Unit extends TemperatureUnit> extends Comparable<Temperature<Unit>> {
    value: number,
    unit: Unit,
    toString(): string;
}

export function fromFarenheit(value: number) : Temperature<'F'> {
    return new TemperatureValue(value, 'F');
}

export default function Temperature({ temperature, range }: { temperature: Temperature<'F'>, range?: { min: Temperature<'F'>, max: Temperature<'F'> } }) {
    let klass = '';
    if (range) {
        klass = 'ok';
        if (temperature.compareTo(range.min) < 0 || temperature.compareTo(range.max) > 0) {
            klass = 'error';
        }
    }
    return <div className={klass}>
        {temperature.toString()}
    </div>;
}
