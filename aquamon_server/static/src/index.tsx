import * as React from "react";
import * as ReactDOM from "react-dom";
import { interval, from } from 'rxjs';
import { ajax } from 'rxjs/ajax'
import { map, concatAll } from 'rxjs/operators';

import TemperatureComponent, { fromFarenheit, Temperature } from './components/Temperature';
import Depth from './components/Depth';
import Api, { Status } from './api';

interface WrappedStatus {
    airTempF: Temperature<'F'>;
    ato_pump_on: boolean;
    cooler_on: boolean;
    currentTempF: Temperature<'F'>;
    depth: number;
    heater_on: boolean;
    humidity: number;
    pH: number;
    pump_on: boolean;
}

function wrapApi(status: Status): WrappedStatus {
    return {
        ...status,
        airTempF: fromFarenheit(status.airTempF),
        currentTempF: fromFarenheit(status.currentTempF),
    }
}

interval(1000)
    .pipe(
        map(Api.getStatus),
        concatAll(),
        map(wrapApi)
    )
    .subscribe(update);

function update(status: WrappedStatus) {
    ReactDOM.render(
        <div>
            <Depth value={status.depth} />
            <TemperatureComponent temperature={status.currentTempF} />
        </div>,
        document.getElementById('appRoot')
    );
}

