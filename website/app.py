from flask import Flask, request, render_template
import socket
import struct
import json
from datetime import datetime

app = Flask(__name__)

# List to store temperature and pressure readings
readings = []

# Function to receive UDP data
def receive_data():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(('0.0.0.0', 1234))
    
    # Receive the data
    data, addr = sock.recvfrom(2)  # Receive 2 bytes
    
    # Unpack the data
    temperature, pressure_raw = struct.unpack('BB', data)
    
    sock.close()
    
    return {
        'temperature': temperature,
        'pressure_raw': pressure_raw,
        'timestamp': datetime.now().isoformat()
    }

# Route for receiving data from Pico
@app.route('/data', methods=['POST'])
def receive_data_route():
    data = receive_data()
    # Append data to readings list
    readings.append(data)
    return 'Data received'

# Route for the homepage
@app.route('/')
def index():
    # Get all readings
    return render_template('index.html', readings=readings)

if __name__ == '__main__':
    app.run(debug=True)
