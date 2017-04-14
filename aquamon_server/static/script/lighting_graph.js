/* globals d3 */

window.graph = function(configData, onSchedulePointSelected, onItemAdded, onItemDeleted, onItemUpdated) {
  var svg = d3.select("#lightingGraph"),
    margin = { top: 20, right: 20, bottom: 30, left: 50 },
    width = +svg.attr("width") - margin.left - margin.right,
    height = +svg.attr("height") - margin.top - margin.bottom,
    g = svg.append("g").attr("transform", "translate(" + margin.left + "," + margin.top + ")");

  var parseTime = d3.timeParse("%H:%M");

  var x = d3.scaleTime()
    .rangeRound([0, width])
    .domain([parseTime("00:00"), parseTime("23:59")]);

  var y = d3.scaleLinear()
    .rangeRound([height, 0]);

  var line = d3.line()
    .x(d => x(d.startTime))
    .y(d => y(d.intensity));

  var mapData = function(configData) {
    return configData.map(d => ({ startTime: parseTime(d.startTime), intensity: d.intensity / 255 * 100, orig: d }));
  };

  // x.domain(d3.extent(data, d => d.startTime));
  x.ticks(d3.timeHour.every(1));
  y.domain([0, 100]);

  var xTickFormat = function(d) {
    return d.getHours() % 3 === 0 ? d3.timeFormat("%I%p")(d) : "";
  };

  g.append("g")
      .attr("transform", `translate(0,${height})`)
      .call(d3.axisBottom(x)
              .tickFormat(xTickFormat)
              .ticks(d3.timeHour.every(1)));

  g.append("g")
      .call(d3.axisLeft(y))
    .append("text")
      .attr("fill", "#000")
      .attr("transform", "rotate(-90)")
      .attr("y", 6)
      .attr("dy", "0.71em")
      .attr("text-anchor", "end")
      .text("Intensity (%)");

  var now_g, now_text, now_line;
  var drawData = function(data) {

    svg.on('mousemove', function() {
      var pos = d3.mouse(this),
        posx = pos[0] - margin.left,
        posy = pos[1] - margin.top;
      if (mousedown_node) {
        if (mousedown_node.prev && posx <= x(mousedown_node.prev.startTime)) {
          posx = x(mousedown_node.prev.startTime);
        }
        if (mousedown_node.next && posx >= x(mousedown_node.next.startTime)) {
          posx = x(mousedown_node.next.startTime);
        }
        if (posx < 0) { posx = 0; }
        if (posy < 0) { posy = 0; }
        if (posx > width) { posx = width; }
        if (posy > height) { posy = height; }
        
        if (!mousedown_node.prev || !mousedown_node.next) {
          posy = height;
        }

        mousedown_node.node.attr("cx", posx);
        mousedown_node.node.attr("cy", posy);
        mousedown_node.data.startTime = x.invert(posx);
        mousedown_node.data.intensity = y.invert(posy);
        path.attr("d", line(data));
      }
    });

    var path = g.append("path")
      .attr("class", "line")
      .attr("fill", "none")
      .attr("stroke", "steelblue")
      .attr("stroke-linejoin", "round")
      .attr("stroke-linecap", "round")
      .attr("stroke-width", 2.0)
      .attr("d", line(data));

    var x_now = x(parseTime(d3.timeFormat("%H:%M")(new Date())));

    now_g = g.append("g");

    now_text = now_g.append("text")
        .attr("fill", "#bbb")
        .attr("transform", "rotate(-90)")
        .attr("y", x_now + 3)
        .attr("x", -5)
        .attr("dy", "0.71em")
        .attr("text-anchor", "end")
        .text("Now");
        
    now_line = now_g.append("line")
        .attr("fill", "none")
        .attr("stroke", "#ccc")
        .attr("stroke-dasharray", "5, 5")
        .attr("stroke-width", 2.0)
        .attr("x1", x_now)
        .attr("x2", x_now)
        .attr("y1", 0)
        .attr("y2", height);



    var mousedown_node;

          var mouseup = function() {
            if (mousedown_node) {
              console.log('mouseup');
              d3.event.preventDefault();
              d3.event.stopPropagation();
              var data = mousedown_node.data;
              onItemUpdated(data.orig, d3.timeFormat("%H:%M")(data.startTime), data.intensity / 100 * 255);
              mousedown_node = null;
            } else {
              console.log('mouseup no node');
            }
          };


    var circles = g.selectAll("circle")
        .data(data)
      .enter()
        .append("circle")
          .attr("fill", "white")
          .attr("stroke", "steelblue")
          .attr("stroke-linejoin", "round")
          .attr("stroke-linecap", "round")
          .attr("stroke-width", 2.0)
          .attr("r", 6)
          .attr("cx", d => x(d.startTime))
          .attr("cy", d => y(d.intensity))
          .on('click', function(d) {
            d3.event.stopPropagation();
            onSchedulePointSelected(d.orig);
            circles.attr("stroke", "steelblue");
            d3.select(this).attr("stroke", "#d40");
          })
          .on('mousedown', function(d) {
            d3.event.preventDefault();
            if (d3.event.button !== 0) {
              onItemDeleted(data.indexOf(d), d.orig);
              return;
            }
            var index = data.indexOf(d);
            mousedown_node = {
              node: d3.select(this),
              data: d, 
              prev: data[index - 1],
              next: data[index + 1]
            };
          })
          .on('mouseup', mouseup);

          svg.on('click', function() {
            if (mousedown_node) { 
              console.log("click on mousedown_node");
              mouseup();
              return; 
            }
            var pos = d3.mouse(this), 
              posx = pos[0] - margin.left,
              posy = pos[1] - margin.top;

            var item = data.reduce((last, current) => posx < x(current.startTime) ? last : current);
            var index = data.indexOf(item);

            if (posx > x(data[0].startTime)) {
              index++;
            }
            if (item) {
              onItemAdded(index, item.orig, d3.timeFormat("%H:%M")(x.invert(posx)), y.invert(posy) / 100 * 255);
            }
          });

          svg.on('mouseup', function() {
          });

  };

  drawData(mapData(configData));

  return {
    dataUpdated: function(configData) {
      g.selectAll("circle").remove();
      g.selectAll(".line").remove();
      now_g.remove();
      drawData(mapData(configData));
    },
    updatePreviewLine: function(timeStr) {
      var x_now = x(parseTime(timeStr));
      now_text.attr("y", x_now);
        
      now_line.attr("x1", x_now)
              .attr("x2", x_now);
    }
  };
};

