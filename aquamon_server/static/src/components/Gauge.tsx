import * as React from 'react';
import * as styles from './Gauge.less';

export interface GaugeProps {
    value: number,
    range: { min: number, max: number },
    fullRange?: { min: number, max: number },
    errorPercent?: number,
    suffix?: string 
}

export default function Gauge({ value, range, fullRange = null, suffix = '' }: GaugeProps) {
    const spread = range.max - range.min;
    const { min, max } = fullRange || { min: range.min - spread * .1, max: range.max + spread * .1 };
    
    const angle = Math.max(0, Math.min(180, (value - min) / (max - min) * 180));

    let klass = '';
    if (value < range.min || value > range.max) {
        klass = styles.error;
    }
    return (
        <div className={styles.gauge1 + ' ' + styles.gauge + ' ' + klass}> 
            <div className={styles.mask}>
                <div className={styles.semiCircle}></div>
                <div className={styles.semiCircleMask} style={ { transform: 'rotate(' + angle + 'deg) translate3d(0,0,0)' } }></div>
            </div>
            <h3 className={klass}>{value}</h3>
        </div>
    );
}