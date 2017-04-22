var point_limit = 60*2*12;

/* globals $, d3 */
function linearRegression(x, y){
		var lr = {};
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

// returns slope, intercept and r-square of the line
var history_graph = function(data, chartId, yLabel, data2, y2Label) {
  var sum = function(a) { return a.reduce((a,b) => a + b, 0); };
  var average = function (array) { return sum(array) / array.length; };
  var movingAverage = function(a, n) {
    return a.map((d,i) => {
      var startIndex = Math.max(i-n, 0);
      return {
        timestamp: d.timestamp,
        value: average(a.slice(startIndex, i + 1).map(d => d.value))
      };
    });
  };

  $(chartId).empty();

  var svg = d3.select(chartId),
    margin = { top: 20, right: 20, bottom: 30, left: 50 },
    width = +svg.attr("width") - margin.left - margin.right,
    height = +svg.attr("height") - margin.top - margin.bottom,
    g = svg.append("g").attr("transform", "translate(" + margin.left + "," + margin.top + ")");

  var height2;
  if (data2) {
    height2 = height * 0.3;
    height = height * 0.7;
  }

  var x = d3.scaleTime()
    .rangeRound([0, width]);

  var y = d3.scaleLinear()
    .rangeRound([height, 0]);

  var line = d3.line()
    .x(d => x(d.timestamp))
    .y(d => y(d.value));

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
    .attr("d", line(data));

  g.append("path")
    .attr("class", "line")
    .attr("fill", "none")
    .attr("stroke", "orange")
    .attr("stroke-linejoin", "round")
    .attr("stroke-linecap", "round")
    .attr("stroke-width", 2.0)
    .attr("d", line(movingAverage(data, 10)));
  

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
      .x(d => x(d.timestamp))
      .y(d => y2(d.value));

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
      .attr("d", line2(data2));

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

  var trackingText = g.append("text")
        .attr("fill", "#999")
        .attr("y", 0)
        .attr("x", width - 10)
        .attr("dy", "0.71em")
        .attr("text-anchor", "end")
        .style("opacity", 0)
        .text("");

  svg.on('mousemove', function() {
    var pos = d3.mouse(this),
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

    var value = (Math.round(y.invert(posy) * 10) / 10).toFixed(1);
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
  });
  
};

var mapValues = function(values, index, filter) {
  var parseTime = d3.timeParse("%Y-%m-%dT%H:%M:%S%Z");
  var rows = d3.csvParseRows(values, function(d/*, i*/) {
    var val = d[index];
    if (!val) { return null; }
    return {
      timestamp: parseTime(d[0]),
      value: parseFloat(d[index])
    };
  });
  if (point_limit > 0 && rows.length > point_limit) {
    rows.splice(0, rows.length - point_limit - 1);
  }
  return rows;
};


var update = function() {
  $.get('/api/status/history.csv').done(function(csv) { 
    history_graph(mapValues(csv, 1), '#temperature_graph', "Temp (F)"); 
    history_graph(mapValues(csv, 2), '#depth_graph', "Depth"); 
    history_graph(mapValues(csv, 6), '#air_temperature_graph', "Air Temp (F)", mapValues(csv, 7), "Humidity (%)"); 
  });
};

window.setInterval(update, 30000);
update();

function limit_data(hours) {
    point_limit = 60*2*hours;
    update();
}

$(function() {
  $('#one_week').click(function(e) {
    e.preventDefault();
    limit_data(24*7);
  });

  $('#one_day').click(function(e) {
    e.preventDefault();
    limit_data(24);
  });

  $('#half_day').click(function(e) {
    e.preventDefault();
    limit_data(12);
  });
  $('#quarter_day').click(function(e) {
    e.preventDefault();
    limit_data(6);
  });
});
