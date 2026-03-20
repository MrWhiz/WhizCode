import si from 'systeminformation';

async function test() {
  console.log('Fetching system info...');
  const [cpu, temp, mem, graphics] = await Promise.all([
    si.currentLoad(),
    si.cpuTemperature(),
    si.mem(),
    si.graphics()
  ]);

  console.log('CPU Usage:', cpu.currentLoad);
  console.log('CPU Temp:', temp);
  console.log('Memory:', mem);
  console.log('Graphics:', JSON.stringify(graphics, null, 2));
}

test();
