# redis-caching
Usage:
Clone repository to a `/project` directory and run `sudo docker-compose up` in `/project`

Creates 3 containers:
 - A rust server using tiny_http
 - A MongoDB database
 - A Redis cache

After the rust server compiles and begins, head to `http://localhost:8000`
