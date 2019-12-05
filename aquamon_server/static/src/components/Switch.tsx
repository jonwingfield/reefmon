import * as React from 'react';
import * as styles from './Switch.less';

export interface SwitchProps {
    isOn: boolean;
    label?: string;
    disabled?: boolean;
}

export default function Switch({ isOn, label, disabled = false}: SwitchProps) {

    return (
        <div>
            <span className={styles.label}>{label}</span>
            <div className={styles.switch + ' ' + (isOn ? styles.toggleOn : '') + ' ' + (disabled ? styles.disabled : '')}>
                <div className={styles.toggle} id='switch'>
                    <div className={styles.toggleTextOff}>OFF</div>
                    <div className={styles.glowComp}></div>
                    <div className={styles.toggleButton}></div>
                    <div className={styles.toggleTextOn}>ON</div>
                </div>
            </div>
        </div>
    );
}