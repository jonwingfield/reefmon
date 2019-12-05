import * as React from 'react';
import { Temperature, fromFarenheit } from './Temperature';
import { MinMaxTempTimes } from '../api';

export default function Range({ title, range, didChange }: { title: string, range: MinMaxTempTimes, didChange: (val: MinMaxTempTimes) => void } ) {
    const [state, setState] = React.useState(range); 

    const changeHandler = function(binding: keyof MinMaxTempTimes) {
        return (e: React.ChangeEvent<HTMLInputElement>) => {
            const val: MinMaxTempTimes = Object.assign({}, state);
            if (binding == 'min' || binding == 'max') {
                val[binding] = fromFarenheit(parseFloat(e.target.value));
            } else {
                val[binding] = e.target.value;
            }
            setState(val);
        }
    };

    return (
        <fieldset>
          <h3>{title}</h3>
          <div>
            Min: <input type="number" min="68" max="84" style={{ width:'7em' }} value={state.min.value} onChange={changeHandler('min')} /> 
            at <input type="text" value={state.minTime} onChange={changeHandler('minTime') } />
          </div>
          <div>
            Max: <input type="number" min="68" max="84" style={{ width:'7em'}} value={state.max.value} onChange={changeHandler('max')} /> 
            at <input type="text" value={state.maxTime} onChange={changeHandler('maxTime') } />
          </div>
          <button onClick={e => didChange(state)}>Update</button>
        </fieldset>
    );
}