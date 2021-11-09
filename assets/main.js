const server = "http://127.0.0.1:8000"

async function post_req(input) {
  let req_body = {
    cmd: input
  };
  console.log(req_body);
  let request = await fetch(`${server}/cache`, {
    method: 'POST',
    headers: {
      'Access-Control-Allow-Headers': '*',
      'Access-Control-Allow-Origin' : '*',
    },
    body: JSON.stringify(req_body),
  });
  console.log(request);
  let reader = await request.body.getReader();
  let response_val = await reader.read();
  console.log(String.fromCharCode(...response_val.value));
  return String.fromCharCode(...response_val.value);
}
