import * as React from 'react';

import { combineLatest, Observable, Subject, merge, timer, BehaviorSubject } from 'rxjs';
import { map, concatAll, scan, auditTime } from 'rxjs/operators';

import Api, { Status, TempSettings, DepthSettings, LightingScheduleJson, HistoryValue, statusToHistory } from './api';

import LightingColors from './components/LightingColors';
import Gauge from './components/Gauge';
import Module from './components/Module';
import Switch from './components/Switch';
import TemperatureModule from './components/TemperatureModule';
import DepthModule from './components/DepthModule';

interface AppState {
    tempSettings: TempSettings;
    depthSettings: DepthSettings;
    status: Status;
    lightingSchedule: LightingScheduleJson;
}
 
const tempSettingsInitial$ = new Observable<TempSettings>(subscriber => {
    Api.getTempSettings().then(status => subscriber.next(status));
});

const depthSettings$ = new Observable<DepthSettings>(subscriber => {
    Api.getDepthSettings().then(status => subscriber.next(status));
});

const lightingScheduleInitial$ = new Observable<LightingScheduleJson>(subscriber => {
    Api.getLightingSchedule().then(schedule => subscriber.next(schedule));
});

const historySubject = new BehaviorSubject<HistoryValue[]>([]);
// delay 1 second to let the page settle, this call is computationally expensive
window.setTimeout(() => Api.getHistory().then(value => historySubject.next(value)), 1000);

const history$ = historySubject.pipe(
    scan((acc: HistoryValue[], value: HistoryValue[], index: number) => {
        return acc.concat(value);
    }, [] as HistoryValue[])
);

// history$.subscribe(x => console.log(x));

const status$ = timer(0, 1000)
    .pipe(
        map(Api.getStatus),
        concatAll(),
    );

status$.pipe(auditTime(30000)).subscribe(status => {
    historySubject.next([ statusToHistory(status) ]);
});

const tempSettingsSubject = new Subject<TempSettings>();

tempSettingsSubject.subscribe((tempSettings: TempSettings) => {
    Api.updateTempSettings(tempSettings);
});

const lightingScheduleSubject = new Subject<LightingScheduleJson>();

const tempSettings$ = merge(tempSettingsInitial$, tempSettingsSubject);

export default function App(props: {}) {
    const [state, setState] = React.useState<AppState>(null);

    React.useEffect(() => {
        combineLatest(tempSettings$, depthSettings$, status$, lightingScheduleInitial$)
            .subscribe(([tempSettings, depthSettings, status, lightingSchedule]) => setState({ tempSettings, depthSettings, status, lightingSchedule }));
    }, []); // only run once

    if (state == null) {
        return <div>Loading...</div>
    }
    const { status, depthSettings, tempSettings, lightingSchedule } = state;
    return (
        <div>
            <section>
                <TemperatureModule currentTempF={status.currentTempF} tempSettings={tempSettings} history$={history$} tempSettingsSubject={tempSettingsSubject} />
                <DepthModule depth={status.depth} depthSettings={depthSettings} history$={history$} />
                <Module title="Switches">
                   <Switch label="ATO" isOn={status.ato_pump_on} disabled={true} />
                   <Switch label="Heater" isOn={status.heater_on} disabled={true} />
                   <Switch label="Cooler" isOn={status.cooler_on} disabled={true} />
                   <Switch label="Pump" isOn={status.pump_on} />
                </Module>
                <Module title="Air Temperature">
                    <Gauge value={status.airTempF.value} range={ { min: 65, max: 84 } } />
                    <div>Humidity: {status.humidity}%</div>
                </Module>
            </section>
            <Module title="Schedule">
                <section id="scheduleDetail">
                    <LightingColors intensities={lightingSchedule.schedule[0]} didChange={e => e} />
                </section>
            </Module>
        </div>
    );
}

