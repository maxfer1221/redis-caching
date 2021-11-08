const server = "http://127.0.0.1:8000"

async function post_req(input) {
  let req_body = {
    cmd: input
  };
  let request = await fetch(`${server}/cache`, {
    method: 'POST',
    body: JSON.stringify(req_body),
  });
  let result = await response.json();
  return result.output;
}
