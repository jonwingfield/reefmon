/* globals $, d3, moment */
import * as d3 from 'd3';
import * as moment from 'moment';
import { HistoryValue } from './api';

function linearRegression(x: number[], y: number[]){
		var lr = {
            slope: 0,
            intercept: 0,
            r2: 0,
        };
		var n = y.length;
		var sum_x = 0;
		var sum_y = 0;
		var sum_xy = 0;
		var sum_xx = 0;
		var sum_yy = 0;
		
		for (var i = 0; i < y.length; i++) {
			
			sum_x += x[i];
			sum_y += y[i];
			sum_xy += (x[i]*y[i]);
			sum_xx += (x[i]*x[i]);
			sum_yy += (y[i]*y[i]);
		} 
		
		lr['slope'] = (n * sum_xy - sum_x * sum_y) / (n*sum_xx - sum_x * sum_x);
		lr['intercept'] = (sum_y - lr.slope * sum_x)/n;
		lr['r2'] = Math.pow((n*sum_xy - sum_x*sum_y)/Math.sqrt((n*sum_xx-sum_x*sum_x)*(n*sum_yy-sum_y*sum_y)),2);
		
		return lr;
}

var xTrackingLines: d3.Selection<SVGLineElement, unknown, HTMLElement, any>[] = [];
var xTrackingTexts: d3.Selection<SVGTextElement, unknown, HTMLElement, any>[] = [];

export interface GraphData {
    timestamp: Date;
    value: number;
}

// returns slope, intercept and r-square of the line
var history_graph = function(
    data: GraphData[], chartElement: HTMLElement, yLabel: string, 
    data2?: GraphData[], y2Label?: string, 
    decimalPlaces = 1, avgPoints = 10) {
  var sum = function(a: number[]) { return a.reduce((a,b) => a + b, 0); };
  var average = function (array: number[]) { return sum(array) / array.length; };
  var movingAverage = function(a: GraphData[], n:number) {
    return a.map((d,i) => {
      var startIndex = Math.max(i-n, 0);
      return {
        timestamp: d.timestamp,
        value: average(a.slice(startIndex, i + 1).map(d => d.value))
      };
    });
  };

  while (chartElement.firstChild) { chartElement.removeChild(chartElement.firstChild); }
  chartElement.setAttribute('width', (chartElement.parentElement.clientWidth).toString());

  var svg = d3.select('#' + chartElement.id),
    margin = { top: 20, right: 20, bottom: 30, left: 50 },
    width = +svg.attr("width") - margin.left - margin.right,
    height = +svg.attr("height") - margin.top - margin.bottom,
    g = svg.append("g").attr("transform", "translate(" + margin.left + "," + margin.top + ")");

  var height2:number;
  if (data2) {
    height2 = height * 0.3;
    height = height * 0.7;
  }

  var x = d3.scaleTime()
    .rangeRound([0, width]);

  var y = d3.scaleLinear()
    .rangeRound([height, 0]);

  var line = d3.line()
    .x(d => x((<any>d).timestamp))
    .y(d => y((<any>d).value));

  var xDomain = d3.extent(data, d => d.timestamp);
  x.domain(xDomain);
  y.domain(d3.extent(data, d => d.value));

  g.append("g")
      .attr("transform", `translate(0,${height})`)
      .call(d3.axisBottom(x));

  g.append("g")
      .call(d3.axisLeft(y))
    .append("text")
      .attr("fill", "#000")
      .attr("transform", "rotate(-90)")
      .attr("y", 6)
      .attr("dy", "0.71em")
      .attr("text-anchor", "end")
      .text(yLabel);

  var path = g.append("path")
    .attr("class", "line")
    .attr("fill", "none")
    .attr("stroke", "steelblue")
    .attr("stroke-linejoin", "round")
    .attr("stroke-linecap", "round")
    .attr("stroke-width", 2.0)
    .attr("d", line(<any>data));

  g.append("path")
    .attr("class", "line")
    .attr("fill", "none")
    .attr("stroke", "orange")
    .attr("stroke-linejoin", "round")
    .attr("stroke-linecap", "round")
    .attr("stroke-width", 2.0)
    .attr("d", line(<any>movingAverage(data, avgPoints)));

  var regressionLine = linearRegression(data.map(d => x(d.timestamp)), data.map(d => y(d.value)));
  var max = d3.max(data, d => x(d.timestamp));

  g.append("line")
    .attr("x1", x(data[0].timestamp))
    .attr("y1", regressionLine.intercept)
    .attr("x2", max)
    .attr("y2", (max * regressionLine.slope) + regressionLine.intercept)
    .style("stroke", "black");

  if (data2) {
    console.log(data2);
    var y2 = d3.scaleLinear()
      .rangeRound([height2, 0]);
    var extent = d3.extent(data2, d => d.value);
    var buffer = (extent[1] - extent[0]) * 0.2;
    y2.domain([extent[0], extent[1] + buffer]);

    var line2 = d3.line()
      .x(d => x((<any>d).timestamp))
      .y(d => y2((<any>d).value));

    g.append("g")
      .attr('transform', "translate(0, " + height + ")")
      .call(d3.axisLeft(y2)
            .ticks(4))
      .append("text")
      .attr("fill", "#000")
      .attr("transform", "rotate(-90)")
      .attr("x", -20)
      .attr("y", 6)
      .attr("dy", "0.71em")
      .attr("text-anchor", "end")
      .text(y2Label);

    g.append("path")
      .attr('transform', "translate(0, " + height + ")")
      .attr("class", "line")
      .attr("fill", "none")
      .attr("stroke", "#0b3")
      .attr("stroke-linejoin", "round")
      .attr("stroke-linecap", "round")
      .attr("stroke-width", 2.0)
      .attr("d", line2(<any>data2));

    // g.append("path")
    //   .attr("class", "line")
    //   .attr("fill", "none")
    //   .attr("stroke", "purple")
    //   .attr("stroke-linejoin", "round")
    //   .attr("stroke-linecap", "round")
    //   .attr("stroke-width", 2.0)
    //   .style("opacity", 0.6)
    //   .attr("d", line2(movingAverage(data2, 10)));
  }

  var trackingLine = g.append("line")
    .attr("x1", 0)
    .attr("y1", 0)
    .attr("x2", width)
    .attr("y2", 0)
    .style('opacity', 0)
    .style("stroke", "#ddd");

  xTrackingLines.push(g.append("line")
    .attr("x1", 0)
    .attr("y1", 0)
    .attr("x2", 0)
    .attr("y2", data2 ? height + height2 : height)
    .style('opacity', 0)
    .style("stroke", "#ddd"));

  xTrackingTexts.push(g.append("text")
        .attr("fill", "#999")
        .attr("y", 0)
        .attr("x", 0)
        .attr("dy", "0.71em")
        .attr("text-anchor", "start")
        .style('text-shadow', '-1px -1px 0 #fff, 1px -1px 0 #fff, -1px 1px 0 #fff, 1px 1px 0 #fff')
        .style("opacity", 0)
        .text(""));

  var trackingText = g.append("text")
        .attr("fill", "#999")
        .attr("y", 0)
        .attr("x", width - 10)
        .attr("dy", "0.71em")
        .attr("text-anchor", "end")
        .style('text-shadow', '-1px -1px 0 #fff, 1px -1px 0 #fff, -1px 1px 0 #fff, 1px 1px 0 #fff')
        .style("opacity", 0)
        .text("");

  svg.on('mousemove', function() {
    var pos = d3.mouse(<any>this),
      posx = pos[0] - margin.left,
      posy = pos[1] - margin.top;
     
    if (posy < 0 ||
      posy > height) {
      trackingLine.style('opacity', 0);
      trackingText.style('opacity', 0);
    } else {
      trackingLine.style('opacity', 1);
      trackingText.style('opacity', 1);
    }

    if (posx < 0 || posx > width) {
      xTrackingLines.forEach(xTrackingLine => xTrackingLine.style('opacity', 0));
      xTrackingTexts.forEach(xTrackingText => xTrackingText.style('opacity', 0));
    } else {
      xTrackingLines.forEach(xTrackingLine => xTrackingLine.style('opacity', 1));
      xTrackingTexts.forEach(xTrackingText => xTrackingText.style('opacity', 1));
    }

    var xValue = moment(x.invert(posx)).format("h:mm a");
    var value = (Math.round(y.invert(posy) * Math.pow(10, decimalPlaces)) / Math.pow(10, decimalPlaces)).toFixed(decimalPlaces);
    if (data2) {
      if (posy > height && posy < height + height2) {
        trackingLine.style('opacity', 1);
        trackingText.style('opacity', 1);
        value = (Math.round(y2.invert(posy-height) * 10) / 10).toFixed(1);
      }
    }


    trackingLine
      // .style('opacity', 1)
      .attr('y1', posy)
      .attr('y2', posy);
    trackingText
      .attr('y', posy - 15)
      .text(value);

    xTrackingLines.forEach(xTrackingLine => 
      xTrackingLine
        .attr('x1', posx)
        .attr('x2', posx));
    xTrackingTexts.forEach(xTrackingText => 
      xTrackingText
        .attr('x', posx + 3)
        .text(xValue));
  });
  
};

export function graph(data: GraphData[], element: HTMLElement, yLabel: string) {
    history_graph(data,
        element,
        yLabel);
}

