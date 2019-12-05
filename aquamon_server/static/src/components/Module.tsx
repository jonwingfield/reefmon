import * as React from 'react';
import { FunctionComponent } from 'react';
import { CombineLatestSubscriber } from 'rxjs/internal/observable/combineLatest';
import * as styles from './Module.less';

export interface ModuleProps<E, S, EProps extends E, SProps extends S> { // it's ugly, but we need the EProps, SProps to ensure type safety.
    title: string;
    expandedComponent?: {
        component: React.FunctionComponent<E>;
        props: EProps;
    },
    settingsComponent?: {
        component: React.FunctionComponent<S>;
        props: SProps;
    }
}

const Module =  <E extends object, S extends object, EProps extends E, SProps extends S>(props: React.PropsWithChildren<ModuleProps<E, S, EProps, SProps>>) => {
    const [expanded, setExpanded] = React.useState<boolean>(false);
    const [inSettingsMode, setSettingsMode] = React.useState<boolean>(false);

    let expandedComponent: any = '';
    if (props.expandedComponent && expanded) {
        expandedComponent = <props.expandedComponent.component {...props.expandedComponent.props} />
    }

    let settingsComponent: any = '';
    if (props.settingsComponent && inSettingsMode) {
        settingsComponent = <props.settingsComponent.component {...props.settingsComponent.props} />
    }

    return (
        <div className={styles.module + ' ' + (expanded ? "n1" : "n3")}>
            <header>
                {props.settingsComponent &&
                    <button onClick={() => { setExpanded(false); setSettingsMode(!inSettingsMode) }}>*</button>
                }
                {props.expandedComponent &&
                    <button onClick={() => { setExpanded(!expanded); setSettingsMode(false); }}>{
                        expanded ? '-' : '+'}</button>
                }
            </header>
            <h3 className={styles.title}>{props.title}</h3>
            {props.children}

            { expanded && expandedComponent }
            { inSettingsMode && settingsComponent }
        </div>
    );
};

export default Module;