import * as React from "react";
import { compare } from "../utils";
import { DepthSettings } from "../api";

var formatInchFraction = function(value: number) {
    var top = Math.abs(Math.round( (Math.round(value * 100.0) % 100) / 100 * 16) );
    var bottom = 16;
    while (top >= 2 && top % 2 === 0) {
        top /= 2;
        bottom /= 2;
    }

    var whole = value >= 0 ? Math.floor(value) : Math.ceil(value);
    let result: string;
    if (whole === 0) {  
        if (top === 0) {
          return "0";
        } else {
          result = value < 0 ? "-" : " "; 
        }
    } else {
        result = whole + " ";
    }

    if (top === 0) {
        return result + '"';
    }
    if (top === bottom) {
        return result + top + '"';
    }

    return result + "" + top + "/" + bottom + '"';
};

function calculateRange(value: number, depthSettings: DepthSettings) {
    const depthValues = depthSettings.depthValues;
    // calculate the range in inches
    var inchesPerStep = depthValues.highInches / (depthValues.high - depthValues.low);
    var depthInches = (value - depthSettings.maintainRange.high) * inchesPerStep;
    return depthInches;
}

export default function Depth({ value, depthSettings}: { value: number, depthSettings: DepthSettings }) {
    let klass = "ok";

    if (compare(value, depthSettings.maintainRange.low) < 0 || compare(value, depthSettings.maintainRange.high) > 0) {
        klass = "error";
    }

    return <div><span className={klass + " " + "big"}>{formatInchFraction(calculateRange(value, depthSettings))}</span> ({value})</div>;
}