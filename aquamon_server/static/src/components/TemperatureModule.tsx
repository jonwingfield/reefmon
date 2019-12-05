import * as React from 'react';
import Module from './Module';
import Gauge from './Gauge';
import { Temperature } from './Temperature';
import { TempSettings, HistoryValue, MinMaxTempTimes } from '../api';
import { Observable, Subject } from 'rxjs';
import { graph } from '../graph';
import HistoryGraph from './HistoryGraph';
import Range from './Range';

export interface TemperatureModuleProps {
    currentTempF: Temperature<'F'>;
    tempSettings: TempSettings;
    history$: Observable<HistoryValue[]>;
    tempSettingsSubject: Subject<TempSettings>;
}

const TemperatureGraph = (props: { history$: Observable<HistoryValue[]> }) => {
    const onUpdate = (history: HistoryValue[], element: HTMLElement) =>
            graph(history.map(x => ({ timestamp: x.timestamp.toDate(), value: x.tempF.value })), 
                element, 'Temp (F)');

    return <HistoryGraph history$={props.history$} id="temperature_graph" onUpdate={onUpdate} />;
}

const Settings = (props: { tempSettings: TempSettings, tempSettingsSubject: Subject<TempSettings>}) => {
    const handleChange = function(key: keyof TempSettings) {
        return (minMax: MinMaxTempTimes) => {
            const tempSettings = Object.assign({}, props.tempSettings);
            tempSettings[key] = minMax;
            props.tempSettingsSubject.next(tempSettings)
        };
    };

    return (
        <React.Fragment>
            <section>
                <Range title="Heater Range" range={props.tempSettings.heater} didChange={handleChange('heater')} />
            </section>
            <section>
                <Range title="Cooler Range" range={props.tempSettings.cooler} didChange={handleChange('cooler')} />
            </section>
        </React.Fragment>
    );
}

export default function TemperatureModule({ currentTempF, tempSettings, history$, tempSettingsSubject }: TemperatureModuleProps) {

    return (
        <React.Fragment>
            <Module title="Temperature" expandedComponent={ { component: TemperatureGraph, props: { history$ } } }
                settingsComponent={ { component: Settings, props: { tempSettings, tempSettingsSubject }}} >
                <Gauge value={currentTempF.value} range={{ min: tempSettings.heater.min.value, max: tempSettings.cooler.max.value }} />
            </Module>
        </React.Fragment>
    );
}
