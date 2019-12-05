import * as React from 'react';
import { Intensities } from '../model/Lighting';

export default function LightingColors({intensities, didChange }: { intensities: Intensities, didChange: (intensities: Intensities) => void }) {
  const state = intensities;

  const handleInput = (index: number) => {
    return (e: React.FormEvent<HTMLInputElement>) => {
      const newState = Object.assign({}, state, { intensities: state.intensities.slice(0) });
      if (index === -1) {
        newState.intensity = parseInt(e.currentTarget.value, 10);
      } else {
        newState.intensities[index] = parseInt(e.currentTarget.value, 10);
      }
      didChange(newState);
    };
  };

  return (
    <section id="colorChange">
      <fieldset id="colorSliders">
        <ColorSlider className="color-uv" value={state.intensities[0]} onInput={handleInput(0)} />
        <ColorSlider className="color-rb" value={state.intensities[1]} onInput={handleInput(1)} />
        <ColorSlider className="color-blue" value={state.intensities[2]} onInput={handleInput(2)} />
        <ColorSlider className="color-cw" value={state.intensities[3]} onInput={handleInput(3)} />
        <ColorSlider className="color-nw" value={state.intensities[4]} onInput={handleInput(4)} />
        <ColorSlider className="color-r" value={state.intensities[5]} onInput={handleInput(5)} />
        <ColorSlider className="color-g" value={state.intensities[6]} onInput={handleInput(6)} />
      </fieldset>
      <fieldset id="intensitySliders">
        <span>
          <ColorSlider className="color-intensity" value={state.intensity} onInput={handleInput(-1)} />
        </span>
      </fieldset>
    </section>
  )
}

function ColorSlider({ className, value, onInput }: { className: string, value: number, onInput: (event: React.FormEvent<HTMLInputElement>) => void }) {
  const getPercent = (value: number) => Math.round(value / 255 * 100.0) + '%';

  return (
    <span>
      <input type="range" min="0" max="255" className={className + " vertical"} value={value} onInput={onInput} />
      <span className="percent">{getPercent(value)}</span>
    </span>
  );
}