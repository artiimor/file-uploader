# file-uploader
Server for file uploads

## Probando los endpoints
- Para obtener un token:
  `curl -s http://localhost:3000/upload_url \
  -H "Authorization: Bearer <my_bearer_token>"` `

- Para probar la subida:
  `curl -s -i -X PUT http://localhost:3000/upload/<my_token>   -F "file=<my_file>"`
