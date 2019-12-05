import { LightingScheduleJson } from "../api";

export interface Intensities {
    intensities: number[];
    intensity: number;
}

export class LightingSchedule {
    json: LightingScheduleJson;

    constructor(json: LightingScheduleJson) {
        this.json = json;
    }


}