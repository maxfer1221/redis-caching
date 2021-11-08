const server = "http://127.0.0.1:8000"

async function post_req(input) {
  console.log(input);
  let req_body = {
    cmd: input
  };
  let request = await fetch(`${server}/cache`, {
    method: 'POST',
    headers: {
        'Access-Control-Allow-Headers': '*',
        'Access-Control-Allow-Origin' : '*',
    },
    body: JSON.stringify(req_body),
  });
  console.log(request);
  let result = await request.json();
  console.log(result);
  return result.output;
}
