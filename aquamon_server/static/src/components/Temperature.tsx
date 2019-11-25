import * as React from "react";

enum TemperatureUnit {
    C = 'C',
    F = 'F'
}

export interface Temperature<Unit extends 'C' | 'F'> {
    value: number,
    unit: Unit,
    toString: () => string;
}

export function fromFarenheit(value: number) : Temperature<'F'> {
    return { value, unit: 'F', toString() { return this.value + this.unit; } };
}

let tc: Temperature<'F'> = fromFarenheit(22.9);

export default function Temperature(props: { temperature: Temperature<'F'> }) {
    return <div>
        {props.temperature.toString()}
    </div>;
}
