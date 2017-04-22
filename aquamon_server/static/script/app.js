/* globals $, moment */
$(function() {
  var liveMode = false;

  $.getJSON('/api/settings/lighting/schedule').then(function(response) {
    var configData = response.schedule;

    $('#colorsliders input').on('input', function() {
      $('#colorsliders input').each(function(index) {
        if (index < 6) {
           selected.intensities[index] = parseInt($(this).val(), 10);
        }
        setPercent(this);
      }).get();
      
      if (liveMode) {
        updateLive(selected);
      }
    });

    $('#intensitySlider').on('input', function() {
      selected.intensity = $(this).val();
      setPercent(this);
      graph.dataUpdated(configData);
      if (liveMode) {
        updateLive(selected);
      }
    });

    var updateLive = function(data) {
        console.log("Previewing: ", data.startTime, data.intensity, data.intensities);
        return $.ajax({
          type: 'POST',
          url: '/api/lighting/live',
          data: JSON.stringify({ lights: data }),
          contentType: 'application/json'
        });
    };

    var scheduleChanged = function() {
      return $.ajax({
        type: 'POST',
        url: '/api/settings/lighting/schedule',
        data: JSON.stringify({ schedule: configData }),
        contentType: 'application/json'
      });
    };

    $('#saveButton').click(function() {
      scheduleChanged();
    });

    var parse = function(time) { return moment(time, 'HH:mm'); };
    var weighted_intensities = function(data) {
      return data.intensities.map(i => Math.round(i * data.intensity / 255));
    };
    var interpolate = function(time, a, b) {
      var total_minutes = parse(b.startTime).diff(parse(a.startTime)) / 60000;
      var elapsed_minutes = parse(time).diff(parse(a.startTime)) / 60000;
      var percent_elapsed = elapsed_minutes / total_minutes;

      return {
        intensities: a.intensities.map((ai, i) => Math.round(ai + (b.intensities[i] - ai) * percent_elapsed)),
        intensity: Math.round(a.intensity + (b.intensity - a.intensity) * percent_elapsed),
        startTime: time
      };
    };

    var previewMode = false;
    $('#preview').click(function() {
      previewMode = true;
      liveMode = false;
      var active = configData[0];
      var nextIndex = 1;
      var startTime = active.startTime;

      var previewTick = function() {
        var interpolated = interpolate(startTime, active, configData[nextIndex]);
        graph.updatePreviewLine(startTime);
        updateSliders(interpolated);
        var ajax = updateLive(interpolated);

        startTime = parse(startTime).add(parseInt($('#previewSpeed').val(), 10), 'm').format("HH:mm");
        if (parse(startTime).isSameOrAfter(parse(configData[nextIndex].startTime))) {
          active = configData[nextIndex];
          nextIndex++;
          if (!configData[nextIndex]) { 
            updateLive(active);
            window.setTimeout(function() { 
              previewMode = false;
              liveMode = false;
              graph.updatePreviewLine(moment().format('HH:mm'));
              updateSliders(selected);
            }, 10);
          }
        } 

        ajax.done(function() {
          window.setTimeout(previewTick, 10);
        });
      };

      previewTick();
    });

    window.setInterval(function() { 
      if (previewMode) { return; }
      graph.updatePreviewLine(moment().format('HH:mm'));
    }, 60000);

    $('#liveMode').on('change', function() {
      liveMode = $(this).is(':checked');
      if (liveMode) {
        updateLive(selected);
      }
    });

    var setPercent = function(el) {
      $(el).closest('span').find('.percent').html(Math.round($(el).val() / 255 * 100) + '%');
    };

    var updateSliders = function(data)  {
      $('#intensitySlider').val(data.intensity);
      setPercent($('#intensitySlider')[0]);
      $('#colorSliders input[type=range]').each(function(index) {
        $(this).val(data.intensities[index]);
        setPercent(this);
      });
      selected = data;
      $('#startTime').html(data.startTime);
      if (liveMode) {
        updateLive(selected);
      }
    };

    var selected = configData[0];
    updateSliders(selected);

    var onSchedulePointSelected = function(data) {
      updateSliders(data);
    };

    var onItemAdded = function(index, prevItem, time, intensity) {
      var data = $.extend(true, {}, prevItem);
      if (index === 0 || index === configData.length) {
        intensity = 0;
      } else { 
        data.intensity = Math.round(intensity, 0);
      }
      data.startTime = time;
      configData.splice(index, 0, data); 
      graph.dataUpdated(configData);
      selected = data;
      updateSliders(data);
      $('#intensitySlider').val(data.intensity);
      console.log("onItemAdded", configData, arguments);
    };

    var onItemUpdated = function(data, time, intensity) {
      data.intensity = Math.round(intensity, 0);
      data.startTime = time;
      $('#intensitySlider').val(data.intensity);
      console.log("onItemUpdated", configData, arguments);
    };

    var onItemDeleted = function(index, data) {
      configData.splice(index, 1);
      graph.dataUpdated(configData);
      $('#intensitySlider').val(data.intensity);
      console.log("onItemDeleted", configData, arguments);
    };

    var graph = window.graph(configData, onSchedulePointSelected, onItemAdded, onItemDeleted, onItemUpdated);

    $('#lightingGraph').on('contextmenu', function() { return false; });

  });

  var round = function(num, places) {
    return Math.round(num*10*places) / 10*places;
  };

  var formatInchFraction = function(value) {
    var top = Math.abs(Math.round( (Math.round(value * 100.0) % 100) / 100 * 16) );
    var bottom = 16;
    while (top >= 2 && top % 2 === 0) {
      top /= 2;
      bottom /= 2;
    }

    var whole = value >= 0 ? Math.floor(value) : Math.ceil(value);
    if (whole === 0) {  
      if (top === 0) {
        return "0";
      } else {
        whole = value < 0 ? "-" : " "; 
      }
    } else {
      whole = whole + " ";
    }

    return whole + "" + top + "/" + bottom + '"';
  };

  var updateStatus = function() {
    $.getJSON('/api/status').then(function(data) {
      var temp = $('#current_temp').html(data.currentTempF + 'F');
      var depth = $('#current_depth').html(data.depth);
      $('#airTemp').html(data.airTempF + 'F');
      $('#humidity').html(data.humidity + '%');

      var waterLevelLow = getIntValue('#waterLevelLow'),
        waterLevelHigh = getIntValue('#waterLevelHigh'),
        minTemp = getFloatValue('#temperatureMin') - 0.30,
        maxTemp = getFloatValue('#temperatureMax') + 0.30,
        minDepthValue = getIntValue('#minDepthValue'),
        maxDepthValue = getIntValue('#maxDepthValue'),
        maxDepthInches = getFloatValue('#maxDepthInches');

      var inchesPerStep = maxDepthInches / (maxDepthValue - minDepthValue);
      var depthInches = (data.depth - waterLevelHigh) * inchesPerStep;
      depth = $('#current_depth_inches').html(formatInchFraction(depthInches));

      $('#steps_per_sixteenth').html(round(1/inchesPerStep/16, 1)); 
      $('#rangeInches').html(formatInchFraction((waterLevelHigh - waterLevelLow) * inchesPerStep).trim());

      if (data.depth < waterLevelLow || data.depth > waterLevelHigh) {
        depth.removeClass('inRange').addClass('outOfRange');
      } else {
        depth.addClass('inRange').removeClass('outOfRange');
      }
      if (data.currentTempF < minTemp || data.currentTempF > maxTemp) {
        temp.removeClass('inRange').addClass('outOfRange');
      } else {
        temp.addClass('inRange').removeClass('outOfRange');
      }
    });
  };
  window.setInterval(updateStatus, 1000);

  var getFloatValue = function(id) {
    return parseFloat($(id).val());
  };

  var getIntValue = function(id) {
    return parseInt($(id).val(), 10);
  };

  $('#updateTempSettings').click(function(e) {
    e.preventDefault();
    var settings = { min: getFloatValue('#temperatureMin'), max: getFloatValue('#temperatureMax') };
    $.post('/api/settings/temperature', JSON.stringify(settings));
  });

  $.getJSON('/api/settings/temperature').then(function(data) {
    $('#temperatureMin').val(data.min);
    $('#temperatureMax').val(data.max);
  });

  $('#updateDepthSettings').click(function(e) {
    e.preventDefault();
    var settings = { 
      maintainRange: { low: getIntValue('#waterLevelLow'), high: getIntValue('#waterLevelHigh') },
      depthValues: { low: getIntValue('#minDepthValue'), high: getIntValue('#maxDepthValue'), highInches: getFloatValue('#maxDepthInches'), tankSurfaceArea: getIntValue('#tankSurfaceArea'), pumpGph: getFloatValue('#pumpGph') }
    };

    $.post('/api/settings/depth', JSON.stringify(settings));
  });

  $.getJSON('/api/settings/depth').then(function(data) {
    $('#waterLevelLow').val(data.maintainRange.low);
    $('#waterLevelHigh').val(data.maintainRange.high);
    $('#minDepthValue').val(data.depthValues.low);
    $('#maxDepthValue').val(data.depthValues.high);
    $('#maxDepthInches').val(data.depthValues.highInches);
    $('#tankSurfaceArea').val(data.depthValues.tankSurfaceArea);
    $('#pumpGph').val(data.depthValues.pumpGph);

    updateStatus();
  });
});
