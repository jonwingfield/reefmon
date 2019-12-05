import { Temperature, fromFarenheit } from './components/Temperature';
import { compose, postData } from './utils';
import * as moment from 'moment';
import { from } from 'rxjs';

interface RawStatus {
    airTempF: number;
    ato_pump_on: boolean;
    cooler_on: boolean;
    currentTempF: number;
    depth: number;
    heater_on: boolean;
    humidity: number;
    pH: number;
    pump_on: boolean;
}

interface MinMaxTimes {
    min: number;
    minTime: string;
    max: number;
    maxTime: string;
}

export interface MinMaxTempTimes {
    min: Temperature<'F'>;
    minTime: string;
    max: Temperature<'F'>;
    maxTime: string;
}

interface RawTempSettings {
    heater: MinMaxTimes;
    cooler: MinMaxTimes;
}

export interface TempSettings {
    heater: MinMaxTempTimes;
    cooler: MinMaxTempTimes;
}

export interface Status {
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

export interface DepthSettings {
    maintainRange: { low: number, high: number},
    depthValues: {
        low: number, 
        high: number,
        highInches: number;
        tankSurfaceArea: number;
        tankVolume: number;
        pumpGph: number;
    }
}

export interface LightingSchedulePoint {
    intensities: number[];
    intensity: number;
    startTime: string;
}

export interface LightingScheduleJson {
    schedule: LightingSchedulePoint[];
}

export interface HistoryValue {
    timestamp: moment.Moment;
    tempF: Temperature<'F'>;
    depth: number;
    heaterOn: boolean;
    atoOn: boolean;
    coolerOn: boolean;
    airTempF: Temperature<'F'>;
    humidity: number;
}

function wrapApi(status: RawStatus): Status {
    return {
        ...status,
        airTempF: fromFarenheit(status.airTempF),
        currentTempF: fromFarenheit(status.currentTempF),
    }
}

function wrapTempSettings(settings: RawTempSettings): TempSettings {
    const wrap = (x: MinMaxTimes) => {
        return {
            ...x,
            min: fromFarenheit(x.min),
            max: fromFarenheit(x.max),
        };
    }
    return {
        cooler: wrap(settings.cooler),
        heater: wrap(settings.heater),
    }
}

function unwrapTempSettings(settings: TempSettings): RawTempSettings {
    const wrap = (x: MinMaxTempTimes) => {
        return {
            ...x,
            min: x.min.value,
            max: x.max.value,
        };
    }
    return {
        cooler: wrap(settings.cooler),
        heater: wrap(settings.heater),
    }
}

export function statusToHistory(status: Status): HistoryValue {
    return {
        timestamp: moment(),
        tempF: status.currentTempF,
        depth: status.depth,
        heaterOn: status.heater_on,
        atoOn: status.ato_pump_on,
        coolerOn: status.cooler_on,
        airTempF: status.airTempF,
        humidity: status.humidity,
    };
}

function mapHistory(csv: string): HistoryValue[] {
    const lines = csv.split("\n").splice(1); // throw away the first line, it might have bad data
    return lines.map(line => {
        const values = line.split(',');
        const [tempF, depth, airTempF, humidity] =
            [parseFloat(values[1]), parseInt(values[2], 10), parseFloat(values[6]), parseFloat(values[7])];
        if (isNaN(tempF) || isNaN(depth) || isNaN(airTempF) || isNaN(humidity)) {
            return null;
        }
        return <HistoryValue>{
            timestamp: moment(values[0]),
            tempF: fromFarenheit(tempF),
            depth: depth,
            heaterOn: new Boolean(values[3]),
            atoOn: new Boolean(values[4]),
            coolerOn: new Boolean(values[5]),
            airTempF: fromFarenheit(airTempF),
            humidity: humidity,
        };
    }).filter(x => x !== null);
}

const Api = {
    getStatus() : Promise<Status> {
        return fetch('/api/status').then(x => <Promise<RawStatus>>x.json()).then(wrapApi);
    },

    getTempSettings() : Promise<TempSettings> {
        return fetch('/api/settings/temperature').then(x => x.json()).then(wrapTempSettings);
    },

    getDepthSettings(): Promise<DepthSettings> {
        return fetch('/api/settings/depth').then(x => x.json());
    },

    getLightingSchedule(): Promise<LightingScheduleJson> {
        return fetch('/api/settings/lighting/schedule').then(x => x.json());
    },

    updateTempSettings(data: TempSettings): Promise<any> {
        console.log("Updating Temp Settings: ", data);
        return postData('/api/settings/test', unwrapTempSettings(data));
    },

    getHistory(hours: number = 12) : Promise<HistoryValue[]> {
        return fetch(`/api/status/history.csv?hours=${hours}`)
            .then(csv => csv.text())
            .then(mapHistory);
    }
};

export default Api;