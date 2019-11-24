/* globals $, moment */
$(function() {
  var liveMode = false;
  var viewingMode = false;

  $.getJSON('/api/settings/lighting/schedule').then(function(response) {
    var configData = response.schedule;

    $('#colorsliders input').on('input', function() {
      $('#colorsliders input').each(function(index) {
        if (index < 6) {
           selected.intensities[index] = parseInt($(this).val(), 10);
        }
        setPercent(this);
      }).get();
      
      if (liveMode || viewingMode) {
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

    var updateLive = function(data, on) {
        console.log("Previewing: ", data.startTime, data.intensity, data.intensities);
        return $.ajax({
          type: 'POST',
          url: '/api/lighting/live',
          data: JSON.stringify({ lights: data, on: !!on }),
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
      updateLive(selected, liveMode);
    });

    var updateToggles = function(toggles) {
      return $.ajax({
        url: '/api/toggles/',
        type: 'POST',
        contentType: 'application/json',
        data: JSON.stringify(toggles),
      });
    };

    $('#pumpToggle').on('change', function() {
      updateToggles({ pump: $(this).is(':checked') });
    });

    var enableViewingMode = function(on) {
        if (viewingMode === on) { return; }
        viewingMode = on;
        if (viewingMode) {
          selectedOld = selected;
          selected = $.extend({}, selected, true);
          console.log("Viewing Mode: ", selected.startTime, selected.intensity, selected.intensities);
          $('#lightingGraph').hide();
        } else { 
          console.log("Viewing Mode off");
          selected = selectedOld;
          $('#lightingGraph').show();
        }

        return $.ajax({
          type: 'POST',
          url: '/api/viewingMode/',
          data: JSON.stringify({ lights: selected, on: viewingMode }),
          contentType: 'application/json'
        });
    }

    $('#viewingMode').on('change', function() {
      enableViewingMode($(this).is(':checked'));
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
    var selectedOld;
    updateSliders(selected);

    var onSchedulePointSelected = function(data) {
      enableViewingMode(false);
      updateSliders(data);
    };

    var onItemAdded = function(index, prevItem, time, intensity) {
      enableViewingMode(false);
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
      enableViewingMode(false);
      data.intensity = Math.round(intensity, 0);
      data.startTime = time;
      $('#intensitySlider').val(data.intensity);
      console.log("onItemUpdated", configData, arguments);
    };

    var onItemDeleted = function(index, data) {
      enableViewingMode(false);
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

    if (top === 0) {
      return whole + '"';
    }
    if (top === bottom) {
      return whole + top + '"';
    }

    return whole + "" + top + "/" + bottom + '"';
  };

  if (window.navigator.serviceWorker) {
    window.navigator.serviceWorker.register('../worker.js');
  }
  var notify = function(message, tag) {
    if (window.Notification) {
      window.Notification.requestPermission().then(function(permission) {
          if (permission === 'granted') {
            navigator.serviceWorker.ready.then(function(registration) {
              registration.showNotification(message, {
                requireInteraction: true,
                vibrate: [200, 100, 200, 100, 200, 100, 400],
                tag: tag,
              });
            });
          } else {
            // alert(message);
          }
      });
    } else {
      // alert(message);
    }
  };

  var updateStatus = function() {
    $.getJSON('/api/status').then(function(data) {
      var temp = $('#current_temp').html(data.currentTempF + 'F');
      var depth = $('#current_depth').html(data.depth);
      var pH = $('#pH').html(data.pH);
      $('#airTemp').html(data.airTempF + 'F');
      $('#humidity').html(data.humidity + '%');
      $('#status\\.heater_on').html(data.heater_on.toString());
      $('#status\\.cooler_on').html(data.cooler_on.toString());
      $('#status\\.ato_pump_on').html(data.ato_pump_on.toString());
      $('#pumpToggle').prop('checked', data.pump_on);

      var waterLevelLow = getIntValue('#waterLevelLow'),
        waterLevelHigh = getIntValue('#waterLevelHigh'),
        minTemp = getFloatValue('#temperatureHeaterMin') - 0.40,
        maxTemp = getFloatValue('#temperatureCoolerMax') + 0.40,
        minDepthValue = getIntValue('#minDepthValue'),
        maxDepthValue = getIntValue('#maxDepthValue'),
        maxDepthInches = getFloatValue('#maxDepthInches');

      var inchesPerStep = maxDepthInches / (maxDepthValue - minDepthValue);
      var depthInches = (data.depth - waterLevelHigh) * inchesPerStep;
      depth = $('#current_depth_inches').html(formatInchFraction(depthInches));

      $('#steps_per_sixteenth').html(round(1/inchesPerStep/16, 1)); 
      $('#rangeInches').html(formatInchFraction((waterLevelHigh - waterLevelLow) * inchesPerStep).trim());

      if (data.depth < waterLevelLow - 10 || data.depth > waterLevelHigh + 10) {
        notify("Water level out of range: " + data.depth, 'water_level');
        depth.removeClass('inRange').addClass('outOfRange');
      } else {
        depth.addClass('inRange').removeClass('outOfRange');
      }
      if (data.currentTempF < minTemp || data.currentTempF > maxTemp) {
        notify("Temperature out of range: " + data.currentTempF + "F", 'temperature');
        temp.removeClass('inRange').addClass('outOfRange');
      } else {
        temp.addClass('inRange').removeClass('outOfRange');
      }
      if (data.pH <= 7.9 || data.pH > 8.40) {
        pH.addClass('outOfRange').removeClass('inRange');
      } else {
        pH.removeClass('outOfRange').addClass('inRange');
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
    var settings = { 
      heater: {
        min: getFloatValue('#temperatureHeaterMin'), 
        minTime: $('#temperatureHeaterMinTime').val(),
        max: getFloatValue('#temperatureHeaterMax'),
        maxTime: $('#temperatureHeaterMaxTime').val(),
      },
      cooler: {
        min: getFloatValue('#temperatureCoolerMin'), 
        minTime: $('#temperatureCoolerMinTime').val(),
        max: getFloatValue('#temperatureCoolerMax'),
        maxTime: $('#temperatureCoolerMaxTime').val(),
      }
    };
    $.post('/api/settings/temperature', JSON.stringify(settings));
  });

  $.getJSON('/api/settings/temperature').then(function(data) {
    $('#temperatureHeaterMin').val(data.heater.min);
    $('#temperatureHeaterMinTime').val(data.heater.minTime);
    $('#temperatureHeaterMax').val(data.heater.max);
    $('#temperatureHeaterMaxTime').val(data.heater.maxTime);
    $('#temperatureCoolerMin').val(data.cooler.min);
    $('#temperatureCoolerMinTime').val(data.cooler.minTime);
    $('#temperatureCoolerMax').val(data.cooler.max);
    $('#temperatureCoolerMaxTime').val(data.cooler.maxTime);
  });

  $('#updateDepthSettings').click(function(e) {
    e.preventDefault();
    var settings = { 
      maintainRange: { low: getIntValue('#waterLevelLow'), high: getIntValue('#waterLevelHigh') },
      depthValues: { low: getIntValue('#minDepthValue'), high: getIntValue('#maxDepthValue'), highInches: getFloatValue('#maxDepthInches'), tankSurfaceArea: getIntValue('#tankSurfaceArea'), pumpGph: getFloatValue('#pumpGph'), tankVolume: getFloatValue('#tankVolume') }
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
    $('#tankVolume').val(data.depthValues.tankVolume);
    $('#pumpGph').val(data.depthValues.pumpGph);

    updateStatus();
    updateMaxDose();
  });

  /** 
   * Dosing 
   */

  var calculateMaxDose = function(volume) { return 0.7 * volume / 10.0; };

  var updateMaxDose = function() {
    var volume = getFloatValue('#tankVolume');
    $('#calculatedMaxDose').html(calculateMaxDose(volume).toFixed(2));
  };

  $('#tankVolume').on('change', updateMaxDose);

  $('#updateDosing').click(function(e) {
    e.preventDefault();
    var doseAmountMl = getFloatValue('#doseAmount');
    var maxDose = calculateMaxDose(getFloatValue('#tankVolume'));
    var numDoses = Math.ceil(doseAmountMl / maxDose);

    var startHour = getIntValue('#doseRangeStart'),
        endHour = getIntValue('#doseRangeEnd'),
        doseSpacing = Math.round((endHour - startHour) / numDoses);

    var schedule = [];
    if (numDoses > 0) {
      schedule.push({
        startTime: startHour + ":00",
        doseAmountMl: (doseAmountMl / numDoses).toFixed(2),
      });
    }
    if (numDoses > 1) {
      schedule = schedule.concat(Array(numDoses - 1).fill(null).map(function(_, i) {
        return { 
          startTime: (startHour + (i+1) * doseSpacing) + ":00",
          doseAmountMl: (doseAmountMl / numDoses).toFixed(2),
        };
      }));
    }

    console.log("Calculated dosage", schedule);

    var settings = {
      pumpRateMlMin: getFloatValue('#pumpRate'),
      doseAmountMl: doseAmountMl,
      doseRangeStart: startHour,
      doseRangeEnd: endHour,
      schedule: schedule,
    };

    $.post('/api/settings/doser', JSON.stringify(settings));
  });

  $.getJSON('/api/settings/doser').then(function(data) {
    $('#pumpRate').val(data.pumpRateMlMin);
    $('#doseAmount').val(data.doseAmountMl);
    $('#doseRangeStart').val(data.doseRangeStart);
    $('#doseRangeEnd').val(data.doseRangeEnd);
  });

  $('#gdo').on('click', function() {
    $.post('/api/gdo/', "{}");
  });
});
