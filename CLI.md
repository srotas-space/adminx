# Use environment variables
export MONGODB_URL="mongodb://localhost:27017"
export ADMINX_DB_NAME="adminx"
adminx create -u admin -e admin@example.com -y

# Use command line arguments
adminx --mongodb-url "mongodb://localhost:27017" --database-name "adminx" list

# Interactive mode (will prompt for connection details)
adminx create -u newuser -e user@example.com

# Quick setup with defaults (localhost:27017, database: adminx)
adminx --mongodb-url "mongodb+srv://username:password@mongo-atlas-cluster.mongodb.net/?retryWrites=true&w=majority&appName=cluster-name" --database-name "dbname" create -u admin -e admin@srotas.space -p password -y

