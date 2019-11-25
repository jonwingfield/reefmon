export interface Status {
    airTempF: number;
    ato_pump_on: boolean;
    cooler_on: boolean;
    currentTempF: number;
    depth: number;
    heater_on: boolean;
    humidity: number;
    pH: number;
    pump_on: boolean;
}

const Api = {
    getStatus() : Promise<Status> {
        return fetch('/api/status').then(response => response.json());
    }
};

export default Api;