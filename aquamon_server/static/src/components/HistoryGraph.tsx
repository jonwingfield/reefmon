import * as React from 'react';
import Module from './Module';
import Gauge from './Gauge';
import { Temperature } from './Temperature';
import { TempSettings, HistoryValue } from '../api';
import { Observable } from 'rxjs';
import { graph } from '../graph';

export interface HistoryGraphProps {
    history$: Observable<HistoryValue[]>,
    onUpdate: (history: HistoryValue[], element: HTMLElement) => void;
    id: string;
    width?: number;
    height?: number;
}

export default function HistoryGraph({ history$, onUpdate, id, width = 300, height=300 }: HistoryGraphProps) {
    const graphEl = React.useRef(null);

    const [history, setHistory] = React.useState<HistoryValue[]>([]);

    React.useEffect(() => {
        const subscription = history$.subscribe(history => {
            setHistory(history);
        })
        return () => subscription.unsubscribe();
    }, []);

    React.useLayoutEffect(() => {
        if (history.length > 0) {
            onUpdate(history, graphEl.current);
        }
    });

    return ( 
        <div>
            <svg ref={graphEl} id={id} width={width} height={height}></svg>
        </div>
    );
}