import * as React from 'react';
import Module from './Module';
import Gauge from './Gauge';
import { HistoryValue, DepthSettings } from '../api';
import { Observable } from 'rxjs';
import { graph } from '../graph';
import HistoryGraph from './HistoryGraph';

export interface DepthModuleProps {
    depth: number;
    depthSettings: DepthSettings;
    history$: Observable<HistoryValue[]>;
}

const DepthGraph = (props: { history$: Observable<HistoryValue[]> }) => {
    const onUpdate = (history: HistoryValue[], element: HTMLElement) =>
            graph(history.map(x => ({ timestamp: x.timestamp.toDate(), value: x.depth })), 
                element, 'Depth');

    return <HistoryGraph history$={props.history$} id="depth_graph" onUpdate={onUpdate} />;
}

export default function DepthModule({ depth, depthSettings, history$ }: DepthModuleProps) {
    return (
        <React.Fragment>
            <Module title="Depth" expandedComponent={ { component: DepthGraph, props: { history$ } } }>
                <Gauge value={depth} range={ { min: depthSettings.maintainRange.low, max: depthSettings.maintainRange.high + 5 } }
                fullRange={ { min: depthSettings.maintainRange.low - 40, max: depthSettings.maintainRange.high + 10 } } />
            </Module>
        </React.Fragment>
    );
}
